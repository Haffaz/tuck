# Synthetic fixtures

Short, invented documents with an expected subject (`Work`, `Invoices`, `Health`, `Education`,
`Warranties`, `Travel`) and a few deliberately unclear or two-record files with no subject.
`diverse` and `diverse-heldout` were written for the earlier pipeline and are used as a held-out
set for the card pipeline: `evals/cards/from_fixtures.py` converts them into the corpus layout.
