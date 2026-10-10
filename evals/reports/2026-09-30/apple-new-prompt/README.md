# New-prompt document recognition benchmark

Same classifier prompt for all models; practical, relaxed document-type scoring requested by the user. 26 fresh local Apple Intelligence calls plus 52 previously recorded Gemma new-prompt calls, rescored consistently. No old-prompt comparison.

## Results

| Model | Input | Accepted /13 | Fresh subset /3 | Errors | Median successful request |
|---|---|---:|---:|---:|---:|
| Apple Intelligence | text | 11/13 | 2/3 | 2 | 0.74 s |
| Apple Intelligence | image | 4/13 | 2/3 | 0 | 0.82 s |
| Gemma E2B | text | 13/13 | 3/3 | 0 | 1.35 s |
| Gemma E2B | image | 5/13 | 2/3 | 0 | 5.82 s |
| Gemma E4B | text | 12/13 | 3/3 | 0 | 2.8 s |
| Gemma E4B | image | 9/13 | 3/3 | 0 | 7.91 s |

Apple text: all 11 non-refused answers were accepted; two refusals count as failures. The refusals occurred on a synthetic residence-permit decision and a real power of attorney. Rental Agreement was returned in Swedish (Hyresavtal): accepted for identity, but an English-output instruction deviation. Insurance document is accepted as a broad family label.

Apple image: many multi-page inputs returned Unknown. On real examples it also called an order confirmation Invoice and an application acknowledgement Email. Image results concern this exact mix of packed pages and individual images, not all vision workflows.

Gemma E2B text now reaches 13/13 because Insurance Policy and Decision are accepted. E4B text reaches 12/13: Membership Agreement still changes the purpose of the order confirmation. These are rescored historical calls, not new Gemma inference.

## Practical scoring

- Exact wording and optional subtypes are not required. Insurance Policy, Decision, Booking, and their more specific equivalents can be useful matches.
- Insurance document is accepted as a broad family label; it is less informative than Insurance Policy.
- A different purpose remains an error: application versus decision, acknowledgement versus approval, invoice/agreement versus order confirmation.
- Document package, Email, and generic administrative wrappers do not identify the substantive document type. Unknown is correct only on the intentionally unidentifiable sample.
- Correct non-English identity is counted separately from following the English-output instruction.
- Rubric was relaxed before Apple inference. Newly observed synonymous/broad labels were manually adjudicated afterward under that policy, recorded in adjudications.json. Same decisions apply to every model.
- Prior reports are preserved as historical records of their stricter scoring. This report supersedes their scores for the current practical acceptance policy.

## Labels by document

### Apple Intelligence

| Case | Expected | Text | Image |
|---|---|---|---|
| s1-D1 | Immigration Decision | [refused] | Unknown |
| s2-D1 | Employment Certificate | Certificate of Employment ✓ | Unknown |
| s3-D1 | Home Insurance Policy | Insurance document ✓ | Unknown |
| s4-D1 | Bank Statement | Bank account statement ✓ | Unknown |
| s5-D1 | Rental Agreement | Hyresavtal ✓ | Unknown |
| s6-D1 | Unknown | Unknown ✓ | Unknown ✓ |
| r1-D1 | Immigration Decision | Decision ✓ | Unknown |
| r2-D1 | Hotel Booking | Booking Confirmation ✓ | Unknown |
| r2-D2 | Membership Confirmation | Order Confirmation ✓ | Invoice |
| r2-D3 | Hotel Booking | Booking Confirmation ✓ | Booking ✓ |
| h1 | Power of Attorney | [refused] | Power of Attorney ✓ |
| h2 | ETA Application Acknowledgement | ETA Acknowledgement ✓ | Email |
| h3 | Taxi Booking | Booking ✓ | Booking Details ✓ |

### Gemma E2B

| Case | Expected | Text | Image |
|---|---|---|---|
| s1-D1 | Immigration Decision | Residence Permit Decision ✓ | Document package |
| s2-D1 | Employment Certificate | Certificate of Employment ✓ | Document package |
| s3-D1 | Home Insurance Policy | Insurance Policy ✓ | Document package |
| s4-D1 | Bank Statement | Bank Account Statement ✓ | Document package |
| s5-D1 | Rental Agreement | Lease Agreement ✓ | Document package |
| s6-D1 | Unknown | Unknown ✓ | Document package |
| r1-D1 | Immigration Decision | Decision ✓ | Application |
| r2-D1 | Hotel Booking | Booking confirmation ✓ | Booking confirmation ✓ |
| r2-D2 | Membership Confirmation | Order Confirmation ✓ | Order Confirmation ✓ |
| r2-D3 | Hotel Booking | Booking confirmation ✓ | Booking confirmation ✓ |
| h1 | Power of Attorney | Power of Attorney/Fullmakt ✓ | Power of Attorney/Fullmakt ✓ |
| h2 | ETA Application Acknowledgement | Acknowledgement of ETA application ✓ | ETA Confirmation |
| h3 | Taxi Booking | Booking Details ✓ | Booking Details ✓ |

### Gemma E4B

| Case | Expected | Text | Image |
|---|---|---|---|
| s1-D1 | Immigration Decision | Residence Permit Decision ✓ | Administrative Record |
| s2-D1 | Employment Certificate | Certificate of Employment ✓ | Certificate of Employment ✓ |
| s3-D1 | Home Insurance Policy | Insurance Policy ✓ | Unknown |
| s4-D1 | Bank Statement | Bank Statement ✓ | Bank Account Statement ✓ |
| s5-D1 | Rental Agreement | Lease Agreement ✓ | Unknown |
| s6-D1 | Unknown | Unknown ✓ | Unknown ✓ |
| r1-D1 | Immigration Decision | Residence Permit Decision ✓ | Residence Permit Decision ✓ |
| r2-D1 | Hotel Booking | Booking Confirmation ✓ | Booking Confirmation ✓ |
| r2-D2 | Membership Confirmation | Membership Agreement | Membership Agreement |
| r2-D3 | Hotel Booking | Booking confirmation ✓ | Booking confirmation ✓ |
| h1 | Power of Attorney | Power of Attorney ✓ | Power of Attorney ✓ |
| h2 | ETA Application Acknowledgement | ETA Application Acknowledgement ✓ | ETA Acknowledgement ✓ |
| h3 | Taxi Booking | Booking Confirmation ✓ | Booking Confirmation ✓ |

## Interpretation

Text is the stronger input on this set. Apple is fastest in this run but refused two documents. Gemma E2B text is the strongest practical candidate in this small sample; 13/13 is not evidence of general perfect accuracy.

Thirteen documents are six authored synthetic packets and seven visually inspected real PDFs. Ten are development cases already seen during prompt design; three were fresh for the preceding A/B experiment. This Apple comparison introduces no new held-out documents. The Unknown prompt example resembles a synthetic case. There is no independent human annotator or LLM judge.

Timings exclude PDF extraction/rendering and neutral warmup. Apple timings include subprocess overhead; Gemma uses a persistent server and prior order/caches. No cold-start, energy, memory, or end-to-end speed comparison was performed. Apple uses its native guided schema, Gemma Ollama JSON schema. Apple context 8192; Gemma 16384. Same prompt and evidence hashes, different runtimes.

The prompt still requests a specific common label; evaluation accepts useful broader answers. Prompt text was kept identical for comparison. One document per call; only documentType is generated. IDs, filenames, destinations and expected labels are not sent as metadata. Existing IDs remain in reused image pixels. No translation, retries, naming, file moves, agent verification, or production integration.

See [prompt.txt](prompt.txt), [rubric.json](rubric.json), [adjudications.json](adjudications.json), and provenance files for details. Personal document contents, images, and source paths are excluded.
