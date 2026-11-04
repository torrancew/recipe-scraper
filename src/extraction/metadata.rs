use super::{
    complete_recipe, html_lines, normalize_lines, selector, trim_ingredient_marker,
    ExtractedRecipe, ExtractionMethod,
};
use scraper::{node::Node, ElementRef, Html};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

const MAX_DEPTH: usize = 64;

pub(super) fn extract(document: &Html) -> Vec<ExtractedRecipe> {
    let values: Vec<Value> = document
        .select(&selector("script[type='application/ld+json']"))
        .filter_map(|script| serde_json::from_str(&script.text().collect::<String>()).ok())
        .collect();
    let mut nodes = Vec::new();
    for value in &values {
        collect_nodes(value, 0, &mut nodes);
    }
    let mut ids = HashMap::new();
    for node in &nodes {
        if let Some(id) = node.get("@id").and_then(Value::as_str) {
            let previous = ids.get(id).copied();
            if previous.map_or(true, |old: &Value| {
                old.as_object().map_or(0, Map::len) < node.as_object().map_or(0, Map::len)
            }) {
                ids.insert(id, *node);
            }
        }
    }
    let json = Metadata { ids, html: true };
    let mut seen = HashSet::new();
    let recipes: Vec<_> = nodes
        .into_iter()
        .filter(|node| has_type(node, "Recipe"))
        .filter_map(|node| {
            let recipe = json.recipe(node, ExtractionMethod::JsonLd)?;
            if let Some(id) = node.get("@id").and_then(Value::as_str) {
                if !seen.insert(id) {
                    return None;
                }
            }
            Some(recipe)
        })
        .collect();
    if !recipes.is_empty() {
        return recipes;
    }

    let mut elements = HashMap::new();
    for element in document.select(&selector("[id]")) {
        if let Some(id) = element.value().attr("id") {
            elements.entry(id).or_insert(element);
        }
    }
    let microdata = Microdata { elements };
    let metadata = Metadata {
        ids: HashMap::new(),
        html: false,
    };
    document
        .select(&selector("[itemscope][itemtype]"))
        .filter(|scope| {
            scope
                .value()
                .attr("itemtype")
                .unwrap_or_default()
                .split_whitespace()
                .any(|kind| schema_type(kind, "Recipe"))
        })
        .filter_map(|scope| {
            let value = microdata.item(scope, 0, &mut Vec::new());
            metadata.recipe(&value, ExtractionMethod::Microdata)
        })
        .collect()
}

fn schema_type(value: &str, name: &str) -> bool {
    value.eq_ignore_ascii_case(name)
        || value.eq_ignore_ascii_case(&format!("https://schema.org/{name}"))
        || value.eq_ignore_ascii_case(&format!("http://schema.org/{name}"))
}

fn has_type(value: &Value, name: &str) -> bool {
    match value.get("@type") {
        Some(Value::String(kind)) => schema_type(kind, name),
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .any(|kind| schema_type(kind, name)),
        _ => false,
    }
}

fn collect_nodes<'a>(value: &'a Value, depth: usize, output: &mut Vec<&'a Value>) {
    if depth >= MAX_DEPTH {
        return;
    }
    match value {
        Value::Object(object) => {
            output.push(value);
            for (key, child) in object {
                if key != "@context" {
                    collect_nodes(child, depth + 1, output);
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_nodes(child, depth + 1, output);
            }
        }
        _ => {}
    }
}

#[derive(Clone, Copy)]
enum Content {
    Text,
    Instructions,
}

struct Metadata<'a> {
    ids: HashMap<&'a str, &'a Value>,
    html: bool,
}

impl Metadata<'_> {
    fn resolve<'a>(&'a self, value: &'a Value) -> &'a Value {
        // resolve bare references without replacing fields already present on a node
        value
            .as_object()
            .filter(|object| object.len() == 1)
            .and_then(|object| object.get("@id"))
            .and_then(Value::as_str)
            .and_then(|id| self.ids.get(id).copied())
            .unwrap_or(value)
    }

    fn recipe(&self, value: &Value, method: ExtractionMethod) -> Option<ExtractedRecipe> {
        let value = self.resolve(value);
        let lines = |field, content| {
            value
                .get(field)
                .map(|value| self.lines(value, content, 0, &mut Vec::new()))
                .unwrap_or_default()
        };
        complete_recipe(ExtractedRecipe {
            title: lines("name", Content::Text).join(" "),
            description: lines("description", Content::Text).join(" "),
            ingredients: lines("recipeIngredient", Content::Text)
                .into_iter()
                .map(|line| trim_ingredient_marker(&line))
                .filter(|line| !line.is_empty())
                .collect(),
            instructions: lines("recipeInstructions", Content::Instructions),
            method,
        })
    }

    fn lines<'a>(
        &'a self,
        value: &'a Value,
        content: Content,
        depth: usize,
        path: &mut Vec<&'a str>,
    ) -> Vec<String> {
        if depth >= MAX_DEPTH {
            return Vec::new();
        }
        match value {
            Value::String(text) => {
                if self.html {
                    html_lines(text)
                } else {
                    // microdata text has already been decoded by the html parser
                    normalize_lines(text)
                }
            }
            Value::Array(values) => values
                .iter()
                .flat_map(|value| self.lines(value, content, depth + 1, path))
                .collect(),
            Value::Object(_) => {
                let id = value.get("@id").and_then(Value::as_str);
                if let Some(id) = id {
                    if path.contains(&id) {
                        return Vec::new();
                    }
                    path.push(id);
                }
                let value = self.resolve(value);
                let output = match content {
                    Content::Text => ["@value", "text", "name", "value"]
                        .iter()
                        .filter_map(|key| value.get(key))
                        .map(|value| self.lines(value, content, depth + 1, path))
                        .find(|lines| !lines.is_empty())
                        .unwrap_or_default(),
                    Content::Instructions => self.instruction(value, depth, path),
                };
                if id.is_some() {
                    path.pop();
                }
                output
            }
            _ => Vec::new(),
        }
    }

    fn instruction<'a>(
        &'a self,
        value: &'a Value,
        depth: usize,
        path: &mut Vec<&'a str>,
    ) -> Vec<String> {
        if let Some(children) = value.get("itemListElement") {
            let mut body = self.lines(children, Content::Instructions, depth + 1, path);
            if !body.is_empty() {
                let mut heading = value
                    .get("name")
                    .map(|name| self.lines(name, Content::Text, depth + 1, path))
                    .unwrap_or_default();
                heading.append(&mut body);
                return heading;
            }
            if !has_type(value, "HowToStep") {
                return body;
            }
        }
        if let Some(item) = value.get("item") {
            return self.lines(item, Content::Instructions, depth + 1, path);
        }
        let mut body = value
            .get("text")
            .or_else(|| value.get("@value"))
            .map(|text| self.lines(text, Content::Text, depth + 1, path))
            .unwrap_or_default();
        let name = value
            .get("name")
            .map(|name| self.lines(name, Content::Text, depth + 1, path).join(" "))
            .unwrap_or_default();
        if !body.is_empty() && !name.is_empty() && !body.join(" ").starts_with(&name) {
            body.insert(0, name);
        }
        body
    }
}

struct Microdata<'a> {
    elements: HashMap<&'a str, ElementRef<'a>>,
}

impl<'a> Microdata<'a> {
    fn item(&self, scope: ElementRef<'a>, depth: usize, path: &mut Vec<ElementRef<'a>>) -> Value {
        if depth >= MAX_DEPTH || path.iter().any(|element| element.id() == scope.id()) {
            return Value::Null;
        }
        path.push(scope);
        let mut properties = Map::new();
        if let Some(kinds) = scope.value().attr("itemtype") {
            properties.insert(
                "@type".to_string(),
                Value::Array(kinds.split_whitespace().map(Value::from).collect()),
            );
        }
        let mut pending: Vec<_> = scope.children().filter_map(ElementRef::wrap).collect();
        if let Some(references) = scope.value().attr("itemref") {
            pending.extend(
                references
                    .split_whitespace()
                    .filter_map(|id| self.elements.get(id).copied()),
            );
        }
        pending.reverse();
        let mut visited = HashSet::new();
        visited.insert(scope.id());
        while let Some(element) = pending.pop() {
            if !visited.insert(element.id()) {
                continue;
            }
            let nested = element.value().attr("itemscope").is_some();
            if let Some(names) = element.value().attr("itemprop") {
                let value = if nested {
                    self.item(element, depth + 1, path)
                } else {
                    property_value(element)
                };
                for name in names.split_whitespace() {
                    let values = properties
                        .entry(name.to_string())
                        .or_insert_with(|| Value::Array(Vec::new()));
                    if let Value::Array(values) = values {
                        values.push(value.clone());
                    }
                }
            }
            if !nested {
                pending.extend(element.children().filter_map(ElementRef::wrap).rev());
            }
        }
        path.pop();
        Value::Object(properties)
    }
}

fn property_value(element: ElementRef<'_>) -> Value {
    let attribute = match element.value().name() {
        "meta" => Some("content"),
        "audio" | "embed" | "iframe" | "img" | "source" | "track" | "video" => Some("src"),
        "a" | "area" | "link" => Some("href"),
        "object" => Some("data"),
        "data" | "meter" => Some("value"),
        "time" => Some("datetime"),
        _ => None,
    };
    let text = attribute
        .and_then(|name| element.value().attr(name))
        .map(str::to_string)
        .unwrap_or_else(|| {
            let mut text = String::new();
            append_property_text(element, &mut text);
            text
        });
    Value::String(text)
}

// hidden elements still carry metadata even when page text extraction skips them
fn append_property_text(element: ElementRef<'_>, output: &mut String) {
    let block = matches!(
        element.value().name(),
        "br" | "p"
            | "div"
            | "li"
            | "ul"
            | "ol"
            | "pre"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "section"
            | "article"
    );
    if block {
        output.push('\n');
    }
    for child in element.children() {
        match child.value() {
            Node::Text(text) => output.push_str(text),
            Node::Element(_) => append_property_text(ElementRef::wrap(child).unwrap(), output),
            _ => {}
        }
    }
    if block {
        output.push('\n');
    }
}
