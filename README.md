# LumenCAT

**Your words. Your workflow. Your control.**

LumenCAT is an offline translation workspace for translators who want to keep their documents private, their terminology consistent, and every editorial decision in their own hands.

Bring your documents and translation memories into one local workspace. Translate segment by segment, reuse relevant references, review quality warnings, and export your work while preserving the original file.

## Why LumenCAT

- **Keep client content on your computer.** Work locally without uploading documents to an external service.
- **Work on your terms.** No account, subscription, or internet connection required.
- **Build consistency into your workflow.** Consult your translation memories and choose which suggestions to use.
- **Stay in control of every sentence.** Edit, confirm, and protect segments yourself.
- **Keep moving through long projects.** Organize documents, track progress, and find the content you need in a focused workspace.

## From source document to reviewed translation

### Translate with focus

Review source and target text side by side in a clear segment editor. Navigate with keyboard shortcuts, confirm completed translations, and lock finished segments against accidental edits. Search your document, replace target text, and revisit changes with undo and redo.

### Reuse the language you trust

Import your translation memories, consult exact and fuzzy matches, and search for words or phrases in context. Apply a reference when it fits your document: you decide what becomes part of the translation.

### Catch avoidable mistakes

Local quality checks flag empty translations, number differences, spacing issues, punctuation differences, and changed or missing URLs, email addresses, and variables. Warnings support your review while leaving the final linguistic judgment to you.

### Protect your work

Automatic saving and persistent edit history help preserve your progress between sessions. Recovery checks help you resume after an interrupted session, and exports create a separate file without replacing your source document.

## Your documents, together

Keep projects, documents, and translation memories organized locally. LumenCAT supports UTF-8 text documents, XLIFF 1.2 with protected inline codes, conservative Microsoft Word DOCX processing, and TMX 1.4/1.4b translation memories with native codes and TU metadata. An optional Windows bridge imports SDLTM and creates updated copies through the official API of an installed Trados Studio; it never overwrites the original. Full format parity remains work in progress: see the [format compatibility execution map](docs/architecture/PARIDAD_FORMATOS.md).

## Built around translator autonomy

Your documents and translation data stay under your control. The current workflow runs entirely on your computer, with no cloud dependency. Translation memory suggestions are references you choose to apply, and quality checks inform your decisions.

LumenCAT is in active development. Document compatibility is still expanding: Word headers, footers, hyperlinks, fields, and revision marks are not translated yet; validate your document workflow before using it for client delivery.

## Technology

Built with **Rust** and **GPUI** for a native desktop experience.
