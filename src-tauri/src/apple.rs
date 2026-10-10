//! Bounded subprocess bridge to Apple's on-device Foundation Models framework.
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

pub(crate) fn invoke(
    helper: &Path,
    input: Option<Value>,
    timeout: Duration,
) -> Result<Value, String> {
    let mut command = Command::new(helper);
    if input.is_none() {
        command.arg("--check");
    }
    let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|_| "Apple Intelligence helper could not start. Requires macOS 26 or later; check Apple Intelligence in System Settings or reinstall Tuck.")?;
    let stdout = child.stdout.take().ok_or("Missing Local model output")?;
    // Concurrent pipe handling keeps the deadline effective even if the helper stalls.
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(128 * 1024)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let mut stdin = child.stdin.take().ok_or("Missing Local model input")?;
    let writer = std::thread::spawn(move || {
        if let Some(input) = input {
            stdin.write_all(input.to_string().as_bytes())?;
        }
        Ok::<_, std::io::Error>(())
    });
    let status = child.wait_timeout(timeout);
    let complete = matches!(&status, Ok(Some(_)));
    if !complete {
        let _ = child.kill();
        let _ = child.wait();
    }
    let written = writer.join().map_err(|_| "Local model input failed")?;
    let output = reader
        .join()
        .map_err(|_| "Local model output failed")?
        .map_err(|_| "Local model output failed")?;
    if !complete {
        return Err("The local model reached its time limit or stopped unexpectedly".into());
    }
    written.map_err(|_| "Local model could not read the request")?;
    if !status.unwrap().unwrap().success() {
        return Err("The local model helper stopped unexpectedly".into());
    }
    let value: Value =
        serde_json::from_slice(&output).map_err(|_| "Invalid response from the local model")?;
    if let Some(error) = value["error"].as_str() {
        return Err(error.chars().take(400).collect());
    }
    Ok(value)
}

#[cfg(test)]
pub fn check(helper: &Path) -> Result<String, String> {
    invoke(helper, None, Duration::from_secs(15))?["status"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "Invalid Apple availability response".into())
}

pub(crate) fn description_request(text: &str, actions: &[&str], completed: &[Value]) -> Value {
    let chars: Vec<_> = text.chars().collect();
    let (excerpt, quotes) = if chars.len() > 6000 {
        let head = chars[..4000].iter().collect::<String>();
        let tail = chars[chars.len() - 2000..].iter().collect::<String>();
        // Collect source quotes before adding application metadata: the omission
        // marker is never evidence from the document.
        let quotes = quote_candidates(&format!("{head}\n{tail}"));
        (
            format!(
                "{head}\n[Middle of excerpt omitted; request a page/chunk if necessary.]\n{tail}"
            ),
            quotes,
        )
    } else {
        (text.to_owned(), quote_candidates(text))
    };
    json!({"operation":"describe", "text":excerpt,
        "quotes":quotes, "actions":actions, "completed":completed})
}

// Spread candidate quotes across the available text, including long OCR lines.
// The original page text remains in the evidence ledger for interpretation.
fn quote_candidates(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    for line in text.lines().filter(|l| {
        !l.trim().starts_with("[Page ")
            && !l.trim().starts_with("[Document has ")
            && !l.trim().starts_with("[Text truncated]")
            && !l.trim().starts_with("[Earlier excerpt]")
            && !l.trim().starts_with("[Middle of excerpt omitted;")
    }) {
        let chars: Vec<_> = line.trim().chars().collect();
        for chunk in chars.chunks(220) {
            let text: String = chunk.iter().collect();
            if text.trim().chars().count() >= 5 {
                chunks.push(text.trim().to_string());
            }
        }
    }
    if chunks.len() <= 12 {
        return chunks;
    }
    (0..12)
        .map(|i| chunks[i * (chunks.len() - 1) / 11].clone())
        .collect()
}
#[cfg(test)]
fn next_read(document: &Value, completed: &[Value]) -> Value {
    let name = document["filename"].as_str().unwrap_or_default();
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let text = document["document_text"].as_str().unwrap_or_default();
    let used = |action: &str, page: usize| {
        completed
            .iter()
            .any(|v| v["action"] == action && v["page"] == page)
    };
    let length = document["initial_text_characters"].as_u64().unwrap_or(0) as usize;
    if matches!(ext.as_str(), "txt" | "md") && length > 8000 {
        for page in 3..=length.div_ceil(4000).min(50) {
            if !used("read_page", page) {
                return json!({"action":"read_page","page":page});
            }
        }
    }
    if ext == "pdf" {
        let count = text
            .lines()
            .next()
            .and_then(|l| l.strip_prefix("[Document has "))
            .and_then(|s| s.strip_suffix(" pages.]"))
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0)
            .min(50);
        let last = text
            .lines()
            .filter_map(|l| {
                l.strip_prefix("[Page ")
                    .and_then(|s| s.strip_suffix(']'))
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);
        // Re-read a possibly clipped last page, then proceed to an unread page.
        let first = if length > 8000 { last.max(1) } else { last + 1 };
        for page in first..=count {
            if !used("read_page", page) {
                return json!({"action":"read_page","page":page});
            }
        }
    }
    if matches!(
        ext.as_str(),
        "pdf" | "png" | "jpg" | "jpeg" | "heic" | "tiff" | "tif"
    ) && text.chars().filter(|c| c.is_alphabetic()).count() < 60
        && !used("retry_ocr", 1)
    {
        return json!({"action":"retry_ocr","page":1});
    }
    Value::Null
}

#[cfg(test)]
fn clip(value: &mut Value, limit: usize) {
    if let Some(text) = value.as_str() {
        *value = json!(text.chars().take(limit).collect::<String>());
    }
}

#[cfg(test)]
fn request(messages: &[Value], schema: &Value) -> Value {
    // The Apple prompt omits a textual schema; the framework supplies it itself.
    let instructions = messages[0]["content"].as_str().unwrap_or_default();
    let mut evidence = Vec::new();
    for (index, message) in messages.iter().enumerate().skip(1) {
        let content = message["content"].as_str().unwrap_or_default();
        if let Ok(mut value) = serde_json::from_str::<Value>(content) {
            if index == 1 {
                let limit = 8000;
                let length = value["document_text"]
                    .as_str()
                    .unwrap_or_default()
                    .chars()
                    .count();
                clip(&mut value["document_text"], limit);
                clip(&mut value["filename"], 200);
                value["excerpt_truncated"] = json!(length > limit);
                value["initial_text_characters"] = json!(length);
                if let Some(examples) = value["saved_corrections"].as_array_mut() {
                    examples.truncate(2);
                    for example in examples {
                        clip(&mut example["cue"], 160);
                        clip(&mut example["example_title"], 100);
                    }
                }
            } else {
                clip(&mut value["tool_result"]["text"], 4000);
            }
            evidence.push(json!({"role":message["role"],"data":value}));
        } else {
            evidence.push(json!({"role":message["role"],"data":content}));
        }
    }
    let mut document = evidence
        .first()
        .map(|e| e["data"].clone())
        .unwrap_or_else(|| json!({}));
    let mut completed = Vec::new();
    let mut pages = vec![json!({"source":"initial excerpt","text":document["document_text"]})];
    for entry in evidence.iter().skip(1) {
        let data = &entry["data"];
        if let Some(action) = data["completed_tool"].as_str() {
            completed.push(json!({"action":action,"page":data["page"],"query":data["query"]}));
            document["latest_tool_result"] = data["tool_result"].clone();
            if matches!(action, "read_page" | "retry_ocr")
                && data["tool_result"]["text"].is_string()
            {
                pages.push(
                    json!({"source":action,"page":data["page"],"text":data["tool_result"]["text"]}),
                );
                document
                    .as_object_mut()
                    .unwrap()
                    .remove("latest_tool_result");
            }
        }
    }
    if let Some(evidence) = schema["x-understanding"]["evidence"].as_array() {
        for passage in evidence {
            pages.push(json!({"source":"verified description evidence", "page":passage["page"], "text":passage["quote"]}));
        }
    }
    let next_read = next_read(&document, &completed);
    document["evidence_pages"] = json!(pages);
    document.as_object_mut().unwrap().remove("document_text");
    document["completed_tools_do_not_repeat"] = json!(completed);
    let task = if completed.is_empty() {
        "Identify the actual document type from this evidence. Finish unless a referenced later page or chunk is needed."
    } else {
        "Identify the document type using the collected evidence. Finish if it identifies the document. Do not repeat completed tools."
    };
    json!({"instructions": instructions,
        "prompt": format!("{task}\n{document}"),
        "evidenceQuotes": pages.iter().flat_map(|page| quote_candidates(page["text"].as_str().unwrap_or_default())).take(48).collect::<Vec<_>>(),
        "nextRead": next_read,
        "understanding":schema["x-understanding"],
        "toolCount": completed.len(),
        "routingEvidence": format!("Filename: {}\n{}", document["filename"], pages.iter().map(|page| format!("[{} page {}]\n{}", page["source"], page["page"], page["text"].as_str().unwrap_or_default())).collect::<Vec<_>>().join("\n\n")),
        "destinationPrompts": schema["x-destination-prompts"].as_array().cloned().unwrap_or_default(),
        "rename": schema["x-rename"].as_bool().unwrap_or(false),
        "rootRules": schema["x-root-rules"].as_str().unwrap_or_default(),
        "folderRules": schema["x-folder-rules"].as_array().cloned().unwrap_or_default(),
        "sourceFolders": document["source_folders"].as_array().cloned().unwrap_or_default(),
        "folderInstructions": schema["properties"]["folder"]["description"].as_str().unwrap_or("Choose the folder matching the document."),
        "actions":schema["properties"]["action"]["enum"],
        "folders":schema["properties"]["folder"]["enum"]})
}

#[cfg(test)]
mod tests {
    #[test]
    fn description_quotes_never_include_application_omission_markers() {
        let text = format!(
            "Cover sheet: read chunk 5.\n{}\nMaintenance report",
            "x".repeat(7000)
        );
        let request = super::description_request(&text, &["finish", "read_page"], &[]);
        assert!(request["text"]
            .as_str()
            .unwrap()
            .contains("[Middle of excerpt omitted;"));
        for quote in request["quotes"].as_array().unwrap() {
            assert!(text.contains(quote.as_str().unwrap()));
            assert!(!quote
                .as_str()
                .unwrap()
                .contains("[Middle of excerpt omitted;"));
        }
    }

    use super::*;
    #[test]
    fn quotes_cover_the_end_and_long_ocr_lines() {
        let text = (0..40)
            .map(|i| format!("Evidence line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let quotes = quote_candidates(&text);
        assert_eq!(quotes.len(), 12);
        assert_eq!(quotes.last().unwrap(), "Evidence line 39");
        let quotes = quote_candidates(&format!("{}Rental agreement", "x".repeat(440)));
        assert_eq!(quotes.last().unwrap(), "Rental agreement");
    }
    #[test]
    fn evidence_hints_are_bounded_and_do_not_repeat() {
        let document = json!({"filename":"scan.txt", "document_text":"cover", "initial_text_characters":14000});
        assert_eq!(
            next_read(&document, &[]),
            json!({"action":"read_page","page":3})
        );
        let done = vec![
            json!({"action":"read_page","page":3}),
            json!({"action":"read_page","page":4}),
        ];
        assert!(next_read(&document, &done).is_null());
        assert_eq!(
            next_read(
                &json!({"filename":"scan.pdf","document_text":"[Document has 4 pages.]\n[Page 1]\nCover","initial_text_characters":9000}),
                &[]
            )["page"],
            1
        );
        assert!(next_read(
            &json!({"filename":"scan.jpg","document_text":"???"}),
            &[json!({"action":"retry_ocr","page":1})]
        )
        .is_null());
    }
    #[test]
    fn apple_context_is_bounded_and_folder_instructions_are_preserved() {
        let messages = vec![
            json!({"role":"system","content":"Instructions."}),
            json!({"role":"user","content":json!({"document_text":"å".repeat(14000),"filename":"scan.txt","saved_corrections":[]}).to_string()}),
        ];
        let request = request(
            &messages,
            &json!({"x-root-rules":"Keep application bundles together.","properties":{"action":{"enum":["finish"]},"folder":{"enum":["Home"]}}}),
        );
        assert!(request["instructions"]
            .as_str()
            .unwrap()
            .starts_with("Instructions."));
        assert_eq!(request["folders"], json!(["Home"]));
        assert_eq!(request["rootRules"], "Keep application bundles together.");
        let prompt = request["prompt"].as_str().unwrap();
        let evidence: Value = serde_json::from_str(prompt.split_once('\n').unwrap().1).unwrap();
        assert_eq!(
            evidence["evidence_pages"][0]["text"]
                .as_str()
                .unwrap()
                .chars()
                .count(),
            8000
        );
        assert!(prompt.contains("scan.txt"));
        assert_eq!(request["actions"], json!(["finish"]));
    }
    #[test]
    fn followup_uses_returned_page_and_keeps_completed_tools() {
        let messages = vec![
            json!({"role":"system","content":"User preferences: Schema: keep this text"}),
            json!({"role":"user","content":json!({"filename":"scan.txt","document_text":"Cover sheet: see chunk 5.","source_folders":["AGENCY","Supporting files"],"saved_corrections":[]}).to_string()}),
            json!({"role":"assistant","content":json!({"action":"read_page","page":5}).to_string()}),
            json!({"role":"user","content":json!({"completed_tool":"read_page","page":5,"query":"","tool_result":{"text":"Insurance certificate"}}).to_string()}),
        ];
        let request = request(
            &messages,
            &json!({"properties":{"action":{"enum":["finish"]},"folder":{"enum":["Insurance"]}}}),
        );
        let prompt = request["prompt"].as_str().unwrap();
        assert!(prompt.contains("Insurance certificate"));
        assert!(request["routingEvidence"]
            .as_str()
            .unwrap()
            .contains("Insurance certificate"));
        assert_eq!(
            request["sourceFolders"],
            json!(["AGENCY", "Supporting files"])
        );
        assert!(prompt.contains("AGENCY") && prompt.contains("Supporting files"));
        assert!(prompt.contains("\"page\":5"));
        assert!(request["instructions"]
            .as_str()
            .unwrap()
            .starts_with("User preferences: Schema: keep this text"));
        assert!(request["prompt"].as_str().unwrap().contains("Cover sheet"));
    }
    #[test]
    fn stalled_and_invalid_helpers_fail_safely() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("fake-helper");
        std::fs::write(&helper, "#!/bin/sh\nexec /bin/sleep 5\n").unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let start = std::time::Instant::now();
        let error = invoke(
            &helper,
            Some(json!({"input":"x".repeat(200_000)})),
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert!(error.contains("time limit"));
        assert!(start.elapsed() < Duration::from_secs(3));
        std::fs::write(&helper, "#!/bin/sh\nprintf 'invalid JSON'\n").unwrap();
        assert!(check(&helper).unwrap_err().contains("Invalid response"));
        std::fs::write(
            &helper,
            r#"#!/bin/sh
printf '{"error":"Enable Apple Intelligence in System Settings"}'
"#,
        )
        .unwrap();
        assert!(check(&helper).unwrap_err().contains("System Settings"));
    }
    #[test]
    fn missing_helper_returns_actionable_error() {
        assert!(check(Path::new("/no-such-tuck-helper"))
            .unwrap_err()
            .contains("System Settings"));
    }
}
