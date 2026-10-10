# Document languages

Automatic document-language detection, Translation preflight and language-download suggestions are disabled. They produced misleading suggestions and added an extra gate before understanding.

Native PDFKit extraction and Vision OCR remain. Vision can automatically select OCR languages; that is part of reading the image, not a separate language classification shown in Tuck. The original extracted text goes directly to Apple Intelligence. There is no automatic translation or language-pack installation flow.

The model describes documents and suggests names in English. This does not guarantee support for every source language. An actual Apple unsupported-language error, refusal, unreadable extraction or uncertain result leaves the file needing input. Removing preflight does not expand the model's language capabilities.

Old nullable language metadata remains compatible with in-memory/API records but is no longer generated or displayed. See [AI architecture](ai-architecture.md).
