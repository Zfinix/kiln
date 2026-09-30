use super::*;

const BINDINGS: [Binding; 2] = [("q", "quit"), ("space", "pause")];

#[test]
fn footer_joins_bindings_on_one_row() {
    assert_eq!(footer(&BINDINGS).to_string(), "q quit · space pause");
}

#[test]
fn help_aligns_the_keys_column() {
    let rows: Vec<String> = help(&BINDINGS).iter().map(ToString::to_string).collect();
    assert_eq!(rows, ["    q  quit", "space  pause"]);
}
