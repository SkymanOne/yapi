//! Constrained sampling for tools: strict JSON-schema parameters, and OpenAI
//! grammar-constrained custom tools.

use std::collections::HashMap;

use serde_json::{Map, Value};
use yapi_types::message::{Message, ToolDeclaration};

use crate::http::is_truthy;
use crate::transcript::declared_tools;
use crate::validation::schema_types;

/// Keywords no strict mode accepts.
const UNSUPPORTED_KEYS: &[&str] = &[
    "$ref",
    "$defs",
    "definitions",
    "allOf",
    "oneOf",
    "patternProperties",
    "dependentSchemas",
    "dependencies",
    "unevaluatedProperties",
    "propertyNames",
    "contains",
    "prefixItems",
    "not",
    "if",
    "then",
    "else",
];

/// A provider's extra keyword restrictions: `(key, value)` is unsupported.
pub type KeywordCheck = fn(&str, &Value) -> bool;

/// The schema rewritten for strict mode: every property required, optional ones
/// nullable, no additional properties. `Err` names the unsupported construct.
pub fn make_strict(schema: &Value, check: Option<KeywordCheck>) -> Result<Value, String> {
    let mut schema = schema.clone();
    if !schema.is_object() {
        return Err("root schema must have type object".into());
    }
    make_node_strict(&mut schema, check)?;
    if schema.get("type") != Some(&Value::from("object")) {
        return Err("root schema must have type object".into());
    }
    Ok(schema)
}

fn is_structured(schema: &Value) -> bool {
    let Some(object) = schema.as_object() else {
        return false;
    };
    let types = schema_types(schema);
    types.contains(&"object")
        || types.contains(&"array")
        || object.contains_key("properties")
        || object.contains_key("items")
}

fn allows_null(schema: &Value) -> bool {
    let Some(object) = schema.as_object() else {
        return false;
    };
    if schema_types(schema).contains(&"null")
        || object.get("const") == Some(&Value::Null)
        || object
            .get("enum")
            .and_then(Value::as_array)
            .is_some_and(|values| values.contains(&Value::Null))
    {
        return true;
    }
    object
        .get("anyOf")
        .and_then(Value::as_array)
        .is_some_and(|variants| variants.iter().any(allows_null))
}

fn make_node_strict(schema: &mut Value, check: Option<KeywordCheck>) -> Result<(), String> {
    let Some(object) = schema.as_object_mut() else {
        return Err("boolean schemas are unsupported".into());
    };
    for key in UNSUPPORTED_KEYS {
        if object.get(*key).is_some_and(|value| !value.is_null()) {
            return Err(format!("{key} schemas are unsupported"));
        }
    }
    if let Some(check) = check {
        for (key, value) in object.iter() {
            if check(key, value) {
                return Err(format!(
                    "{key}: {} is unsupported",
                    yapi_types::json::stringify(value)
                ));
            }
        }
    }
    if let Some(any_of) = object.get_mut("anyOf") {
        let Some(variants) = any_of
            .as_array_mut()
            .filter(|variants| !variants.is_empty())
        else {
            return Err("anyOf must contain at least one schema".into());
        };
        for variant in variants {
            if is_structured(variant) {
                return Err("object and array unions are unsupported".into());
            }
            make_node_strict(variant, check)?;
        }
    }
    if let Some(items) = object.get_mut("items") {
        if items.is_array() {
            return Err("tuple schemas are unsupported".into());
        }
        make_node_strict(items, check)?;
    }

    let is_object = object.get("type") == Some(&Value::from("object"));
    if object.contains_key("properties") && !is_object {
        return Err("properties require type object".into());
    }
    if !is_object {
        return Ok(());
    }
    if object
        .get("additionalProperties")
        .is_some_and(|value| value != &Value::Bool(false))
    {
        return Err("schema-valued or true additionalProperties is unsupported".into());
    }
    let required: Vec<String> = match object.get("required") {
        None => Vec::new(),
        Some(Value::Array(keys)) if keys.iter().all(Value::is_string) => keys
            .iter()
            .filter_map(|key| key.as_str().map(str::to_owned))
            .collect(),
        Some(_) => return Err("object required must be a string array".into()),
    };
    let mut properties = match object.get("properties") {
        None => Map::new(),
        Some(Value::Object(properties)) => properties.clone(),
        Some(_) => return Err("object properties must be a schema map".into()),
    };
    if required.iter().any(|key| !properties.contains_key(key)) {
        return Err("required contains an unknown property".into());
    }
    let names: Vec<Value> = properties.keys().cloned().map(Value::String).collect();
    for (key, property) in properties.iter_mut() {
        make_node_strict(property, check)?;
        if !required.contains(key) && !allows_null(property) {
            let original = std::mem::take(property);
            *property = serde_json::json!({ "anyOf": [original, { "type": "null" }] });
        }
    }
    if object.contains_key("properties") {
        object.insert("properties".into(), Value::Object(properties));
    }
    object.insert("required".into(), Value::Array(names));
    object.insert("additionalProperties".into(), Value::Bool(false));
    Ok(())
}

/// Whether to send `tool` with strict JSON-schema sampling: it asks for it, the
/// provider supports it and the schema can be made strict. `Err` when the tool
/// requires strict sampling that cannot be had.
pub fn strict_sampling(
    tool: &ToolDeclaration,
    supports_strict: bool,
    check: Option<KeywordCheck>,
) -> Result<bool, String> {
    let Some(config) = tool
        .constrained_sampling
        .as_ref()
        .and_then(Value::as_object)
    else {
        return Ok(false);
    };
    if config.get("type").and_then(Value::as_str) != Some("json_schema") {
        return Ok(false);
    }
    let required = config.get("strict").and_then(Value::as_str) == Some("require");
    if supports_strict {
        return match make_strict(&tool.parameters, check) {
            Ok(_) => Ok(true),
            Err(_) if !required => Ok(false),
            Err(reason) => Err(format!(
                "Tool \"{}\" requires JSON-schema constrained sampling, but {reason}.",
                tool.name
            )),
        };
    }
    if required {
        return Err(format!(
            "Tool \"{}\" requires JSON-schema constrained sampling, but strict tools are unsupported.",
            tool.name
        ));
    }
    Ok(false)
}

/// The parameters to send: strict when `strict`, as declared otherwise.
pub fn tool_parameters(tool: &ToolDeclaration, strict: bool) -> Value {
    if strict {
        make_strict(&tool.parameters, None).unwrap_or_else(|_| tool.parameters.clone())
    } else {
        tool.parameters.clone()
    }
}

/// How to send a tool as an OpenAI grammar-constrained custom tool.
#[derive(Debug, PartialEq)]
pub struct Grammar<'a> {
    /// `lark` or `regex`.
    pub syntax: &'static str,
    /// The grammar, in `syntax`.
    pub definition: &'a str,
    /// The tool's one required string argument, which carries the raw input.
    pub property: &'a str,
}

/// The grammar to send `tool` with: it asks for one and the provider supports
/// OpenAI grammar tools. Lark is preferred over regex. `Err` when the tool asks
/// for a grammar it cannot use.
pub fn grammar_sampling(
    tool: &ToolDeclaration,
    supports_grammar: bool,
) -> Result<Option<Grammar<'_>>, String> {
    let Some(config) = tool
        .constrained_sampling
        .as_ref()
        .filter(|config| config["type"] == "grammar")
    else {
        return Ok(None);
    };
    if !supports_grammar {
        return Ok(None);
    }
    let unusable = |reason: &str| {
        format!(
            "Tool \"{}\" cannot use grammar constrained sampling: {reason}.",
            tool.name
        )
    };
    let variant = |name: &str| {
        config["variants"][name]
            .as_str()
            .filter(|definition| !definition.trim().is_empty())
    };
    let (syntax, definition) = match (variant("openai_lark"), variant("openai_regex")) {
        (Some(lark), _) => ("lark", lark),
        (None, Some(regex)) => ("regex", regex),
        (None, None) => return Err(unusable("no supported grammar variant was provided")),
    };
    let property = grammar_property(&tool.parameters).map_err(|reason| unusable(&reason))?;
    Ok(Some(Grammar {
        syntax,
        definition,
        property,
    }))
}

fn grammar_property(schema: &Value) -> Result<&str, String> {
    if schema["type"] != "object" {
        return Err("grammar constrained sampling requires an object parameter schema".into());
    }
    let Some([Value::String(property)]) = schema["required"].as_array().map(Vec::as_slice) else {
        return Err(
            "grammar constrained sampling requires exactly one required string property".into(),
        );
    };
    let entry = &schema["properties"][property];
    if !is_truthy(entry) {
        return Err(format!(
            "grammar constrained sampling requires a properties entry for {property}"
        ));
    }
    if entry["type"] != "string" {
        return Err(format!(
            "grammar constrained sampling property {property} must have type string"
        ));
    }
    Ok(property)
}

/// The tools a transcript declares that are sent with a grammar, by name, with
/// the argument that carries each one's raw input. `messages` is the
/// transcript as the provider receives it, after
/// [`resolve_transcript`](crate::transcript::resolve_transcript): a model that
/// folds later system messages into the first one sees only the current tools.
pub fn grammar_properties(
    messages: &[Message],
    supports_grammar: bool,
) -> Result<HashMap<String, String>, String> {
    let mut properties = HashMap::new();
    for tool in declared_tools(messages) {
        if let Some(grammar) = grammar_sampling(&tool, supports_grammar)? {
            properties.insert(tool.name.clone(), grammar.property.to_owned());
        }
    }
    Ok(properties)
}

/// The raw input of a grammar tool call, from the argument that carries it.
pub fn grammar_input<'a>(
    name: &str,
    arguments: &'a Map<String, Value>,
    property: &str,
) -> Result<&'a str, String> {
    arguments
        .get(property)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!("Grammar tool call \"{name}\" requires argument \"{property}\" to be a string.")
        })
}

/// A grammar tool call's raw input as it streams, re-encoded as the JSON text
/// of its arguments object.
#[derive(Debug)]
pub struct GrammarInput {
    /// The argument that carries the raw input.
    pub property: String,
    input: String,
    started: bool,
    closed: bool,
}

impl GrammarInput {
    fn new(property: String) -> GrammarInput {
        GrammarInput {
            property,
            input: String::new(),
            started: false,
            closed: false,
        }
    }

    /// The input of a custom tool call to `name`, carried by the argument that
    /// `properties` (from [`grammar_properties`]) gives for it. A call to a tool
    /// sent without a grammar still keeps its input, as `input`.
    pub fn for_call(properties: &HashMap<String, String>, name: &str) -> GrammarInput {
        let property = properties.get(name).map_or("input", String::as_str);
        GrammarInput::new(property.to_owned())
    }

    /// The raw input `arguments` carry.
    pub fn input<'a>(&self, arguments: &'a Map<String, Value>) -> &'a str {
        arguments
            .get(&self.property)
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    /// Sets `arguments` to carry `input`. Returns the JSON text that takes the
    /// arguments streamed so far to it, closing the object when `close`, or
    /// `None` when there is nothing to stream. `Err`, leaving `arguments` as
    /// they are, when `input` does not extend the input streamed so far, or
    /// changes once closed.
    pub fn set(
        &mut self,
        arguments: &mut Map<String, Value>,
        input: String,
        close: bool,
    ) -> Result<Option<String>, String> {
        let delta = self.append(&input, close)?;
        *arguments = Map::from_iter([(self.property.clone(), Value::String(input))]);
        Ok(delta)
    }

    fn append(&mut self, input: &str, close: bool) -> Result<Option<String>, String> {
        if self.closed {
            if close && input == self.input {
                return Ok(None);
            }
            return Err(format!(
                "grammar tool input for property \"{}\" changed after it was closed",
                self.property
            ));
        }
        let Some(added) = input.strip_prefix(self.input.as_str()) else {
            return Err(format!(
                "grammar tool input for property \"{}\" changed non-monotonically",
                self.property
            ));
        };
        if !close && added.is_empty() {
            return Ok(None);
        }
        let mut delta = String::new();
        if !self.started {
            delta.push('{');
            delta.push_str(&yapi_types::json::stringify(&Value::from(
                self.property.as_str(),
            )));
            delta.push_str(":\"");
            self.started = true;
        }
        let quoted = yapi_types::json::stringify(&Value::from(added));
        delta.push_str(&quoted[1..quoted.len() - 1]);
        if close {
            delta.push_str("\"}");
            self.closed = true;
        }
        self.input = input.to_owned();
        Ok(Some(delta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn makes_optional_properties_nullable() {
        let schema = json!({"type":"object","required":["path"],"properties":{
            "path":{"type":"string"},"limit":{"type":"number","description":"n"}}});
        let strict = make_strict(&schema, None).unwrap();
        assert_eq!(
            yapi_types::json::to_string(&strict).unwrap(),
            r#"{"type":"object","required":["path","limit"],"properties":{"path":{"type":"string"},"limit":{"anyOf":[{"type":"number","description":"n"},{"type":"null"}]}},"additionalProperties":false}"#
        );
    }

    #[test]
    fn rejects_unsupported_constructs() {
        assert!(
            make_strict(
                &json!({"type":"object","properties":{"a":{"oneOf":[]}}}),
                None
            )
            .is_err()
        );
        assert!(make_strict(&json!({"type":"object","additionalProperties":true}), None).is_err());
        assert!(make_strict(&json!({"type":"string"}), None).is_err());
    }

    #[test]
    fn resolves_grammar_tools() {
        let tool = |sampling: Value, parameters: Value| ToolDeclaration {
            name: "codemode".into(),
            description: String::new(),
            parameters,
            constrained_sampling: Some(sampling),
        };
        let code =
            json!({"type":"object","required":["code"],"properties":{"code":{"type":"string"}}});
        let both = tool(
            json!({"type":"grammar","variants":{"openai_regex":"a+","openai_lark":"start: \"a\""}}),
            code.clone(),
        );
        assert_eq!(
            grammar_sampling(&both, true).unwrap(),
            Some(Grammar {
                syntax: "lark",
                definition: "start: \"a\"",
                property: "code"
            })
        );
        assert_eq!(grammar_sampling(&both, false).unwrap(), None);
        let regex = tool(
            json!({"type":"grammar","variants":{"openai_lark":" ","openai_regex":"a+"}}),
            code.clone(),
        );
        assert_eq!(
            grammar_sampling(&regex, true).unwrap().unwrap().syntax,
            "regex"
        );
        let empty = tool(json!({"type":"grammar","variants":{}}), code);
        assert_eq!(
            grammar_sampling(&empty, true).unwrap_err(),
            "Tool \"codemode\" cannot use grammar constrained sampling: no supported grammar variant was provided."
        );
        let two = tool(
            json!({"type":"grammar","variants":{"openai_regex":"a+"}}),
            json!({"type":"object","required":["a","b"],"properties":{"a":{"type":"string"},"b":{"type":"string"}}}),
        );
        assert_eq!(
            grammar_sampling(&two, true).unwrap_err(),
            "Tool \"codemode\" cannot use grammar constrained sampling: grammar constrained sampling requires exactly one required string property."
        );
        let unusable = |parameters: Value| {
            grammar_sampling(
                &tool(
                    json!({"type":"grammar","variants":{"openai_regex":"a+"}}),
                    parameters,
                ),
                true,
            )
            .unwrap_err()
        };
        let prefix = "Tool \"codemode\" cannot use grammar constrained sampling: grammar constrained sampling";
        assert_eq!(
            unusable(json!({"type":"string"})),
            format!("{prefix} requires an object parameter schema.")
        );
        assert_eq!(
            unusable(json!({"type":"object","required":["code"],"properties":{}})),
            format!("{prefix} requires a properties entry for code.")
        );
        assert_eq!(
            unusable(
                json!({"type":"object","required":["code"],"properties":{"code":{"type":"number"}}})
            ),
            format!("{prefix} property code must have type string.")
        );
    }

    #[test]
    fn streams_grammar_input_as_json() {
        let mut input = GrammarInput::new("code".into());
        assert_eq!(input.append("", false).unwrap(), None);
        assert_eq!(
            input.append("say(\"hi\")", false).unwrap().as_deref(),
            Some(r#"{"code":"say(\"hi\")"#)
        );
        assert_eq!(
            input.append("say(\"hi\")\n", true).unwrap().as_deref(),
            Some(r#"\n"}"#)
        );
        assert_eq!(input.append("say(\"hi\")\n", true).unwrap(), None);
        assert!(input.append("other", true).is_err());
        let mut input = GrammarInput::new("code".into());
        input.append("ab", false).unwrap();
        assert!(input.append("b", false).is_err());
        let mut input = GrammarInput::new("code".into());
        assert_eq!(
            input.append("", true).unwrap().as_deref(),
            Some(r#"{"code":""}"#)
        );
    }
}
