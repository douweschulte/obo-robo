# Obo-robo

Automate some obo tasks.

Possible commands:
- `lint`: Show general obo errors for a given file
- `fmt`: Format an obo file following the obo standard
- `fix`: Fix with ontology specific rules and format the file
- `newfmt`: Fix a PSI-MOD obo file to follow the obo format better
- `search`: Load an obo file to search it interactively

This parameter only works with `lint` and `fix`:
- `changed_line_numbers`: only lint the items that have seen changes to suppress warnings of untouched item.

This parameter works with `fmt` and `fix`:
- `keep_ordering`: keep the original ordering as much as possible. This still reorders based on line type.

This parameter only works with `fix`:
- `bump`: the author name to bump the version with. This updates the `saved-by` field, the `data-version` version number (patch version bump based on semantic versioning), and sets the `date` field to the current date and time in UTC+0. If `changed_line_numbers` is also supplied it does not change any version (number, author, and date) if the `data-version` or `date` lines have been changed.

# Standard

Follows:
- <https://owlcollab.github.io/oboformat/doc/GO.format.obo-1_4.html>
- <https://owlcollab.github.io/oboformat/doc/obo-syntax.html>

# License

MIT or Apache-2.0

# LLM Usage

No LLMs (or related technologies) have been used in the creation of this software.