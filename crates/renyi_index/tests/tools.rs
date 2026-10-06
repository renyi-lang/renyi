//! The tool manifest (`renyi tools`, decision D6): a function with `expose
//! as tool` gets a JSON Schema built from its types, its purpose and its
//! permissions; a recursive type is written once under `$defs`.

use renyi_index::{manifest_json, tools_of, Tool};
use renyi_syntax::SourceFile;

fn tools(source: &str) -> Vec<Tool> {
    tools_of(&[SourceFile::new("demo.ry", source)])
}

const SOURCE: &str = "module demo
  purpose: Tools for an agent.

import std.http exposing HttpError

public type Currency is Text where value.length() is 3
  purpose: An ISO code.

public type Shape is one of
  purpose: What can be drawn.
  Circle(radius: Decimal)
  Point
end

public type Order
  purpose: One order.
  has id: Integer
  has kind: Text as \"type\"
  has note: maybe Text
  has shape: Shape
end

public type Tree
  purpose: A node with children.
  has label: Text
  has children: List of Tree
end

public function convert(amount: Decimal, source: Currency, target: Currency) returns Decimal
  or fails with HttpError
  needs network.http(\"api.example.com\")
  purpose: Convert an amount between currencies.
  expose as tool

  return amount
end

public function describe(order: Order, labels: Set of Text, counts: Map of Text to Integer) returns Shape
  purpose: Describe an order.
  expose as tool

  return order.shape
end

public function flatten(tree: Tree) returns List of Text
  purpose: Every label of the tree.
  expose as tool

  return [tree.label]
end

function helper() returns Integer
  return 1
end
";

#[test]
fn exposed_functions_become_tools_with_schemas() {
    let tools = tools(SOURCE);
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
    assert_eq!(names, ["demo.convert", "demo.describe", "demo.flatten"]);

    let convert = &tools[0];
    assert_eq!(
        convert.description.as_deref(),
        Some("Convert an amount between currencies.")
    );
    assert_eq!(convert.permissions, ["network.http(\"api.example.com\")"]);
    assert_eq!(convert.fails, ["HttpError"]);
    assert_eq!(convert.line, 29);
    let input = convert.input_schema.render();
    assert!(
        input.contains("\"amount\": {\n      \"type\": \"number\",\n      \"format\": \"decimal\""),
        "{input}"
    );
    // a subtype takes its base's schema
    assert!(
        input.contains("\"source\": {\n      \"type\": \"string\""),
        "{input}"
    );
    assert!(
        input.contains("\"required\": [\n    \"amount\",\n    \"source\",\n    \"target\"\n  ]"),
        "{input}"
    );
    assert!(input.contains("\"additionalProperties\": false"), "{input}");
    let output = convert.output_schema.as_ref().expect("a result").render();
    assert!(output.contains("\"format\": \"decimal\""), "{output}");

    let describe = &tools[1];
    let input = describe.input_schema.render();
    // a record is an object keyed by the field names or the `as` names; a
    // `maybe` field is not required and admits null
    assert!(
        input.contains("\"type\": {\n          \"type\": \"string\""),
        "{input}"
    );
    assert!(input.contains("\"note\": {\n          \"anyOf\": [\n            {\n              \"type\": \"string\"\n            },\n            {\n              \"type\": \"null\"\n            }\n          ]"), "{input}");
    assert!(
        input.contains(
            "\"required\": [\n        \"id\",\n        \"type\",\n        \"shape\"\n      ]"
        ),
        "{input}"
    );
    // a sum type is one object per variant with its name under `kind`
    assert!(input.contains("\"oneOf\""), "{input}");
    assert!(
        input.contains("\"kind\": {\n                  \"const\": \"Circle\""),
        "{input}"
    );
    assert!(input.contains("\"uniqueItems\": true"), "{input}");
    assert!(
        input.contains("\"additionalProperties\": {\n        \"type\": \"integer\""),
        "{input}"
    );
    let output = describe.output_schema.as_ref().expect("a result").render();
    assert!(output.starts_with("{\n  \"oneOf\": ["), "{output}");

    // a recursive type is expanded where it is used and defined once under
    // `$defs`, where the reference inside it points: the structure appears
    // twice (a property and a required entry each time)
    let flatten = &tools[2];
    let input = flatten.input_schema.render();
    assert_eq!(
        input.matches("\"$ref\": \"#/$defs/Tree\"").count(),
        2,
        "{input}"
    );
    assert_eq!(
        input.matches("\"$defs\": {\n    \"Tree\": {").count(),
        1,
        "{input}"
    );
    assert_eq!(input.matches("\"label\"").count(), 4, "{input}");

    let manifest = manifest_json(&tools).render();
    assert!(
        manifest.starts_with("[\n  {\n    \"name\": \"demo.convert\""),
        "{manifest}"
    );
    assert!(manifest.contains("\"output_schema\": {"), "{manifest}");
}

#[test]
fn a_project_without_tools_has_an_empty_manifest() {
    let tools = tools(
        "module demo\n  purpose: No tools.\n\nfunction go() returns Integer\n  return 1\nend\n",
    );
    assert!(tools.is_empty());
    assert_eq!(manifest_json(&tools).render(), "[]\n");
}
