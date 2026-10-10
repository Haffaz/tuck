# Tuck’s product experience

Choose a folder, choose its categories, then Start Tuck. Clear matches are filed automatically. Ask the user only when a decision needs help.

Keep the existing calm green-and-cream shell. The sidebar and header stay fixed; content scrolls. Setup, current status and attention are the main surface. Activity is secondary. Technical explanations, paths and advanced actions belong behind disclosures.

## Folder tree

Overview shows configured categories and AI suggestions in the same expandable hierarchy. Selecting a folder opens its path, rule and enabled setting beside it. Selecting the root opens rules that apply everywhere. Implicit parents are grouping folders until the user adds a rule. Disabled parents visibly mark their descendants off.

The tree preserves rule precedence; display nesting never reorders the underlying array. Edits remain in the existing preference draft until Save rules, and suggested structures stay separate until Use these categories. Discard changes reloads current saved preferences. Adding, removing or changing a category changes filing configuration; it does not rename or delete existing document folders. The tree shows filing destinations, not a live file-system browser. During an active session the tree is available under Folders & rules; saving rules uses the existing stop-and-revalidate behavior.

## A small decision

An attention card asks where a document belongs. Choose a category and File document. Renaming is optional, under More options alongside retrying, rule examples and details. Leave for now leaves the document untouched. Selected ready documents can still be filed together.

The File document action saves an edited suggestion internally, reads back its validated destination and approves that exact destination. A failed update, changed settings or stale document stops the action. Rust still checks revisions, source hashes, folder scope and exclusive target creation. Remember this example may save a correction in the session, but never moves the file or saves a rule without the existing explicit actions.

## One folder now, several later

The MVP still has one active root and one in-memory engine. DocumentDecision receives documents, rules, draft values and actions from its owning folder; it does not read global folder state. The root plus document ID keys each component so local disclosures cannot leak into another folder’s view.

Before enabling multiple roots, introduce a stable folder ID and a folder registry, move session/draft/selection/watch state into a per-folder owner, and pass that ID through every backend operation. Each folder keeps its own .tuck settings and rules. A future folder switcher should be a compact button and popover, only shown when multiple folders exist. The current UI separation prepares that work; it does not implement concurrent roots or durable sessions.

No persistent history, undo, database or cloud synchronization is introduced by this UI refinement.
