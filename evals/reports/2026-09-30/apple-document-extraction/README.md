# Apple structured document extraction experiment

Uses RecognizeDocumentsRequest revision1 on macOS 27. The API is available from macOS 26. All inference and extraction ran locally. Application code was not changed.

## Classification results

Same new classifier prompt, schema, source documents/pages and relaxed label acceptance. The baseline is the previously recorded PDFKit/line-OCR text condition. Structured results are 39 fresh calls. No old-prompt comparison.

| Classifier | Baseline accepted /13 | Structured accepted /13 | Baseline request median | Structured request median |
|---|---:|---:|---:|---:|
| Apple Intelligence | 11/13 | 12/13 | 0.74 s | 0.76 s |
| Gemma E2B | 13/13 | 12/13 | 1.35 s | 1.49 s |
| Gemma E4B | 12/13 | 12/13 | 2.8 s | 3.02 s |

- Apple: both previously refused documents now classified correctly, but an order confirmation became Membership Agreement. Net +1 accepted; zero refusals in this run. This does not establish that refusals are solved.
- Gemma E2B: the intentionally unidentifiable synthetic packet changed from Unknown to Administrative record. Net -1.
- Gemma E4B: retained the Membership Agreement mistake; accepted count unchanged.
- All three models accepted all three previously designated fresh cases. They were fresh during the earlier prompt study, not newly held out for this experiment.
- Apple again returned Rental Agreement in Swedish (Hyresavtal), counted correct for identity but an English-output instruction deviation.
- New equivalent labels ETA Acknowledgement Notice and Permit were accepted under the existing broad-purpose policy, with decisions recorded.

## Extraction results

Across 35 pages: 8 tables and 3 lists detected. All 13 documents extracted successfully.

| Case | Pages | Tables | Lists | Baseline extraction | Structured extraction | Baseline chars | Structured chars |
|---|---:|---:|---:|---:|---:|---:|---:|
| s1-D1 | 4 | 1 | 0 | 0.28 s | 0.71 s | 1707 | 1958 |
| s2-D1 | 4 | 1 | 0 | 0.16 s | 0.59 s | 1690 | 1941 |
| s3-D1 | 4 | 1 | 0 | 0.15 s | 0.62 s | 1659 | 1910 |
| s4-D1 | 4 | 1 | 0 | 0.16 s | 0.60 s | 1692 | 1944 |
| s5-D1 | 4 | 1 | 0 | 0.17 s | 0.60 s | 1683 | 1934 |
| s6-D1 | 4 | 1 | 0 | 0.15 s | 0.60 s | 1668 | 1919 |
| r1-D1 | 4 | 0 | 0 | 1.36 s | 15.87 s | 5642 | 5719 |
| r2-D1 | 2 | 0 | 1 | 0.09 s | 13.81 s | 3925 | 3334 |
| r2-D2 | 1 | 0 | 1 | 0.35 s | 0.31 s | 1601 | 1612 |
| r2-D3 | 1 | 0 | 1 | 0.05 s | 0.38 s | 3276 | 3273 |
| h1 | 1 | 1 | 0 | 0.39 s | 0.43 s | 1547 | 1837 |
| h2 | 1 | 0 | 0 | 0.04 s | 0.25 s | 1016 | 978 |
| h3 | 1 | 1 | 0 | 0.04 s | 0.28 s | 555 | 914 |

Median document extraction: baseline 0.16 s; structured 0.60 s. Totals: baseline 3.38 s; structured 35.05 s.

These are single-pass observed timings, not controlled cold-start speed claims. An earlier synthetic-page pilot took 27.56 seconds inside the structured request, before the scored extraction run. Framework/model initialization and caching can affect timing. Structured extraction renders every selected page and loads once per document; baseline uses the existing per-page subprocess invocation and direct text where available. The two methods do different amounts of work. Model request timings exclude extraction. Memory usage was not profiled.

## Extraction implementation

- PDFs rendered one page at a time with PDFKit at the current reader's 1600 x 2200 bounding size, then passed to RecognizeDocumentsRequest. This experiment deliberately applies recognition even to native-text PDFs to measure the alternative.
- Automatic language detection and language correction enabled. Runtime reported Swedish support; no translation used. See capabilities.json for the detected supported-language list.
- Native paragraphs, detected title, table cells with row/column spans, and lists retained in private intermediate JSON. Only the selected text representation reaches the classifier.
- Experimental serializer emits paragraph text, a Markdown heading for the detected title, HTML tables preserving spans, and list markers. It avoids paragraphs contained inside tables/lists and orders blocks geometrically from top to bottom, then left to right. Complex multiple-column reading order is not validated by this adapter.
- Original page markers preserved. Same source pages as the baseline, no filenames or expected labels supplied. Entire selected text included without truncation.
- No recognized word occurrences were lost between the API transcript and serialized output in any of the 13 cases. This checks serialization coverage, not whether OCR read the original correctly.
- Visually checked the real booking table: 12 rows with 2 cells each retained. Synthetic transmission-note tables also inspected. The full corpus was not independently transcribed or graded for character accuracy.
- No fallbacks or retries were used to improve scored outcomes.

## Returned labels

### Apple Intelligence

| Case | Expected | Baseline | Structured |
|---|---|---|---|
| s1-D1 | Immigration Decision | [refused] | Decision ✓ |
| s2-D1 | Employment Certificate | Certificate of Employment ✓ | Certificate of Employment ✓ |
| s3-D1 | Home Insurance Policy | Insurance document ✓ | Insurance document ✓ |
| s4-D1 | Bank Statement | Bank account statement ✓ | Bank account statement ✓ |
| s5-D1 | Rental Agreement | Hyresavtal ✓ | Hyresavtal ✓ |
| s6-D1 | Unknown | Unknown ✓ | Unknown ✓ |
| r1-D1 | Immigration Decision | Decision ✓ | Decision ✓ |
| r2-D1 | Hotel Booking | Booking Confirmation ✓ | Booking Confirmation ✓ |
| r2-D2 | Membership Confirmation | Order Confirmation ✓ | Membership Agreement |
| r2-D3 | Hotel Booking | Booking Confirmation ✓ | Booking Confirmation ✓ |
| h1 | Power of Attorney | [refused] | Power of Attorney ✓ |
| h2 | ETA Application Acknowledgement | ETA Acknowledgement ✓ | ETA Acknowledgement Notice ✓ |
| h3 | Taxi Booking | Booking ✓ | Booking ✓ |

### Gemma E2B

| Case | Expected | Baseline | Structured |
|---|---|---|---|
| s1-D1 | Immigration Decision | Residence Permit Decision ✓ | Permit ✓ |
| s2-D1 | Employment Certificate | Certificate of Employment ✓ | Certificate of Employment ✓ |
| s3-D1 | Home Insurance Policy | Insurance Policy ✓ | Insurance policy ✓ |
| s4-D1 | Bank Statement | Bank Account Statement ✓ | Bank Account Statement ✓ |
| s5-D1 | Rental Agreement | Lease Agreement ✓ | Lease Agreement ✓ |
| s6-D1 | Unknown | Unknown ✓ | Administrative record |
| r1-D1 | Immigration Decision | Decision ✓ | Decision ✓ |
| r2-D1 | Hotel Booking | Booking confirmation ✓ | Booking confirmation ✓ |
| r2-D2 | Membership Confirmation | Order Confirmation ✓ | Membership Order Confirmation ✓ |
| r2-D3 | Hotel Booking | Booking confirmation ✓ | Booking confirmation ✓ |
| h1 | Power of Attorney | Power of Attorney/Fullmakt ✓ | Power of Attorney/Fullmakt ✓ |
| h2 | ETA Application Acknowledgement | Acknowledgement of ETA application ✓ | ETA Acknowledgement Notice ✓ |
| h3 | Taxi Booking | Booking Details ✓ | Booking Details ✓ |

### Gemma E4B

| Case | Expected | Baseline | Structured |
|---|---|---|---|
| s1-D1 | Immigration Decision | Residence Permit Decision ✓ | Residence Permit Decision ✓ |
| s2-D1 | Employment Certificate | Certificate of Employment ✓ | Certificate of Employment ✓ |
| s3-D1 | Home Insurance Policy | Insurance Policy ✓ | Insurance Policy ✓ |
| s4-D1 | Bank Statement | Bank Statement ✓ | Bank Statement ✓ |
| s5-D1 | Rental Agreement | Lease Agreement ✓ | Lease Agreement ✓ |
| s6-D1 | Unknown | Unknown ✓ | Unknown ✓ |
| r1-D1 | Immigration Decision | Residence Permit Decision ✓ | Residence Permit Decision ✓ |
| r2-D1 | Hotel Booking | Booking Confirmation ✓ | Booking Confirmation ✓ |
| r2-D2 | Membership Confirmation | Membership Agreement | Membership Agreement |
| r2-D3 | Hotel Booking | Booking confirmation ✓ | Booking Confirmation ✓ |
| h1 | Power of Attorney | Power of Attorney ✓ | Power of Attorney ✓ |
| h2 | ETA Application Acknowledgement | ETA Application Acknowledgement ✓ | ETA Acknowledgement Notice ✓ |
| h3 | Taxi Booking | Booking Confirmation ✓ | Booking Confirmation ✓ |

## Recommendation and scope

Use structured Apple recognition as a candidate for scanned images and layout-sensitive pages, retaining direct PDFKit extraction for clean native-text PDFs. The data does not support rendering and OCRing every PDF by default. A hybrid route was not implemented or benchmarked here.

This is a small development experiment: six synthetic packets and seven real PDFs from the authorized test folder. It changes both recognition and structure serialization, so gains cannot be assigned specifically to table markup or the new OCR. Historical baseline inference was not rerun; only baseline extraction was rerun and matched every stored baseline text exactly. No causal claim about refusal reduction or general model superiority follows from one run.

The production reader and installed app remain unchanged. The compiled experimental reader, extraction adapter, fixed prompt and scored results are available here for inspection. All source PDFs, text, images and private paths are excluded from this report.
