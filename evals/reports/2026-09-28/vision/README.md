# Image reading and destination checks — filing-v5

Measured locally on macOS 27.0 using Apple's on-device model. Synthetic document text only is included in these reports.

| Variant | Broader expected outcomes | Focused expected outcomes |
| --- | --- | --- |
| Installed filing-v4 baseline | 9/12 | Not run |
| Expanded reading/routing prompts (rejected) | 8/12 | 5/5 |
| Final narrow changes | 9/12 | 4/5 |

The final broader result matches the baseline on every case: 7/10 substantive files were filed correctly and the two intentionally ambiguous/unreadable inputs remained manual. A medical certificate, AGENCY attachment, and electricity bill still required input. No wrong destination or forbidden title was emitted. Median final latency was about 4.10 seconds; this small regression set is not a general accuracy benchmark.

The final focused set includes a rendered driving-licence PNG and birth-registration PDF. The PDF used image input successfully. Apple rejected the synthetic driving-licence reading with a guardrail error; Tuck preserved the manual fallback without retrying a different modality. The other four expected folder and title checks passed. The earlier expanded prompts passed all five focused cases but worsened the broader set, so those prompts were discarded. Prompt/model sensitivity remains a limitation.

A separate read-only check of the two user-reported images selected Identification & Licensing for both, with image input confirmed. The driving licence received the correct type; the partial birth record retained its original filename because its exact type was not sufficiently supported. SHA-256 checks confirmed both source files were unchanged. No personal text, identifiers, or images are stored in this report.

The final implementation adds the missing Driving Licence and Birth Certificate vocabulary entries, separates folder selection from name validation in the conditional checker, and supplies image attachments only to the document reading stage. Root preferences are separated from folder topic rules. The general reading/routing prompts remain unchanged. 53 Swift support checks passed; the Rust suite previously passed 61 tests with 8 live tests ignored. Older macOS fallback branches compile but were not runtime-tested here.
