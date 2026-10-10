# Gemma classifier prompt A/B test

104 local calls comparing the previous short prompt with the proposed classifier role, purpose and examples prompt. Within each pair only the system prompt changed; both prompts were rerun live. One document per call, output only documentType, with IDs owned by the runner.

## Results

| Model | Input | Old correct /13 | New correct /13 | Old fresh /3 | New fresh /3 | Old median | New median |
|---|---|---:|---:|---:|---:|---:|---:|
| Gemma E2B | text | 5/13 | 11/13 | 2/3 | 3/3 | 1.16 s | 1.35 s |
| Gemma E2B | image | 6/13 | 5/13 | 3/3 | 2/3 | 5.67 s | 5.82 s |
| Gemma E4B | text | 9/13 | 11/13 | 3/3 | 3/3 | 2.16 s | 2.8 s |
| Gemma E4B | image | 6/13 | 9/13 | 3/3 | 3/3 | 7.78 s | 7.91 s |

Correct means the document identity matches the frozen expected type or an equivalent label. Generic labels can be insufficiently specific without being wholly false. Synonym decisions are recorded in adjudications.json and apply equally to both prompts. The fresh PDFs are only three examples, not a reliable general-accuracy estimate. Medians exclude warmup, rendering and OCR; caching can affect them.

## Paired changes

| Model | Input | Improved decisions | Regressed decisions | Unchanged correctness |
|---|---|---:|---:|---:|
| Gemma E2B | text | 6 | 0 | 7 |
| Gemma E2B | image | 0 | 1 | 12 |
| Gemma E4B | text | 3 | 1 | 9 |
| Gemma E4B | image | 4 | 1 | 8 |

## Returned labels

D = previously used development documents; F = fresh PDFs. Six D documents are synthetic four-page packets. Four D and all three F documents are real.

### Gemma E2B

| Case | Split | Expected | Old text | New text | Old image | New image |
|---|---|---|---|---|---|---|
| s1-D1 | D | Immigration Decision | Administrative Record | Residence Permit Decision ✓ | Document | Document package |
| s2-D1 | D | Employment Certificate | Synthetic Document | Certificate of Employment ✓ | Document | Document package |
| s3-D1 | D | Home Insurance Policy | Insurance Document | Insurance Policy | Document | Document package |
| s4-D1 | D | Bank Statement | Synthetic Document | Bank Account Statement ✓ | Document | Document package |
| s5-D1 | D | Rental Agreement | Synthetic Document | Lease Agreement ✓ | Document | Document package |
| s6-D1 | D | Unknown | Administrative Record | Unknown ✓ | Document | Document package |
| r1-D1 | D | Immigration Decision | Decision Letter | Decision | Official Document | Application |
| r2-D1 | D | Hotel Booking | Booking Confirmation ✓ | Booking confirmation ✓ | Booking Confirmation ✓ | Booking confirmation ✓ |
| r2-D2 | D | Membership Confirmation | Order Confirmation ✓ | Order Confirmation ✓ | Order Confirmation ✓ | Order Confirmation ✓ |
| r2-D3 | D | Hotel Booking | Booking Confirmation ✓ | Booking confirmation ✓ | Booking Confirmation ✓ | Booking confirmation ✓ |
| h1 | F | Power of Attorney | Power of Attorney ✓ | Power of Attorney/Fullmakt ✓ | Power of Attorney ✓ | Power of Attorney/Fullmakt ✓ |
| h2 | F | ETA Application Acknowledgement | Acknowledgement Letter | Acknowledgement of ETA application ✓ | ETA Receipt ✓ | ETA Confirmation |
| h3 | F | Taxi Booking | Booking Form ✓ | Booking Details ✓ | Booking Details ✓ | Booking Details ✓ |

### Gemma E4B

| Case | Split | Expected | Old text | New text | Old image | New image |
|---|---|---|---|---|---|---|
| s1-D1 | D | Immigration Decision | Decision | Residence Permit Decision ✓ | Document packet | Administrative Record |
| s2-D1 | D | Employment Certificate | Certificate of Employment ✓ | Certificate of Employment ✓ | Document packet | Certificate of Employment ✓ |
| s3-D1 | D | Home Insurance Policy | insurance policy | Insurance Policy | Document packet | Unknown |
| s4-D1 | D | Bank Statement | Bank Account Statement ✓ | Bank Statement ✓ | Document packet | Bank Account Statement ✓ |
| s5-D1 | D | Rental Agreement | Lease agreement ✓ | Lease Agreement ✓ | Document packet | Unknown |
| s6-D1 | D | Unknown | administrative record | Unknown ✓ | Document packet | Unknown ✓ |
| r1-D1 | D | Immigration Decision | Decision | Residence Permit Decision ✓ | Decision | Residence Permit Decision ✓ |
| r2-D1 | D | Hotel Booking | Booking Confirmation ✓ | Booking Confirmation ✓ | booking confirmation ✓ | Booking Confirmation ✓ |
| r2-D2 | D | Membership Confirmation | Order Confirmation ✓ | Membership Agreement | Order confirmation ✓ | Membership Agreement |
| r2-D3 | D | Hotel Booking | Booking confirmation ✓ | Booking confirmation ✓ | Booking confirmation ✓ | Booking confirmation ✓ |
| h1 | F | Power of Attorney | Power of Attorney ✓ | Power of Attorney ✓ | Power of Attorney ✓ | Power of Attorney ✓ |
| h2 | F | ETA Application Acknowledgement | ETA Acknowledgement ✓ | ETA Application Acknowledgement ✓ | ETA Receipt ✓ | ETA Acknowledgement ✓ |
| h3 | F | Taxi Booking | Booking Confirmation ✓ | Booking Confirmation ✓ | Booking Confirmation ✓ | Booking Confirmation ✓ |

## Recommendation

Keep the new prompt as the stronger text-recognition candidate. Both models reached 11/13, with median request times of 1.35 seconds for E2B and 2.80 seconds for E4B. This sample does not establish a general model winner.

Images need separate validation: E2B declined from 6/13 to 5/13, while E4B improved from 6/13 to 9/13. E4B also regressed on the membership order confirmation, calling it Membership Agreement. Preserve that case as a regression test. Both text variants still produced an overly broad insurance label on the home-insurance fixture.

## Scope and limits

- The new system prompt combines a role, a purpose-based decision rule, specificity guidance and examples. This test measures their combined effect; it does not isolate which element helped.
- Development samples were seen during prompt design. The Unknown example closely resembles one synthetic case. Keep fresh and development scores separate.
- An ETA application acknowledgement must be distinguished from approval. ETA Confirmation alone is treated as ambiguous; it is not counted as an explicit false approval claim.
- Bilingual Power of Attorney/Fullmakt is counted as the right identity but does not strictly follow the English-only instruction.
- Text and images are separate conditions. Packed-page results should not be generalized to single-page images or the production file pipeline.
- All evidence hashes matched within pairs. Input modes and prompt order were alternated across cases and models. Models ran sequentially.
- No file moves, naming, folder assignment, agent review or production code changes. Personal document contents and file paths are excluded from this report.
- Read [METHOD.md](METHOD.md), [old-prompt.txt](old-prompt.txt) and [new-prompt.txt](new-prompt.txt). Settings, model digests and runtime version are in provenance.json.
