//! Parse a [schema.org Recipe](https://schema.org/Recipe) from various commonly observed
//! structures

use std::fmt::{self, Debug, Display};

use accessory::Accessors;
use iso8601_duration::Duration;
use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug, PartialEq)]
pub struct MaybeDuration(Option<Duration>);

#[cfg(test)]
impl MaybeDuration {
    pub(crate) fn from_secs(secs: impl Into<f32>) -> Self {
        Self(Some(Duration::new(0., 0., 0., 0., 0., secs.into())))
    }
}

impl MaybeDuration {
    pub fn duration(&self) -> Option<&Duration> {
        self.0.as_ref()
    }

    pub fn human_readable(&self) -> Option<String> {
        let pp = |d| pretty_duration::pretty_duration(&d, None);
        self.0.and_then(|d| d.to_std().map(pp))
    }
}

impl<'de> Deserialize<'de> for MaybeDuration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(Deserialize::deserialize(deserializer).ok()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum IngredientList {
    Single(String),
    Multi(Vec<String>),
}

impl IntoIterator for IngredientList {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Single(s) => vec![s].into_iter(),
            Self::Multi(v) => v.into_iter(),
        }
    }
}

#[cfg(test)]
impl IngredientList {
    pub(crate) fn multi<S: Into<String>>(ingredients: impl IntoIterator<Item = S>) -> Self {
        Self::Multi(ingredients.into_iter().map(Into::into).collect())
    }

    pub(crate) fn single(ingredient: impl Into<String>) -> Self {
        Self::Single(ingredient.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Instruction {
    Simple(String),
    Structured { text: String },
}

impl Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Simple(s) => s,
            Self::Structured { text } => text,
        };
        Display::fmt(text, f)
    }
}

#[cfg(test)]
impl Instruction {
    pub(crate) fn simple(instruction: impl Into<String>) -> Self {
        Self::Simple(instruction.into())
    }

    pub(crate) fn structured(instruction: impl Into<String>) -> Self {
        let text = instruction.into();
        Self::Structured { text }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Accessors, Deserialize)]
pub struct InstructionSection {
    #[access(get)]
    name: String,
    #[serde(rename = "itemListElement")]
    directions: Vec<Instruction>,
}

impl IntoIterator for InstructionSection {
    type Item = Instruction;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.directions.into_iter()
    }
}

#[cfg(test)]
impl InstructionSection {
    pub(crate) fn new(
        name: impl Into<String>,
        directions: impl IntoIterator<Item = Instruction>,
    ) -> Self {
        Self {
            name: name.into(),
            directions: Vec::from_iter(directions),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum InstructionList {
    Single(Instruction),
    Multi(Vec<Instruction>),
    Sections(Vec<InstructionSection>),
}

impl InstructionList {
    pub fn sections(&self) -> Option<impl Iterator<Item = &InstructionSection>> {
        match self {
            Self::Sections(v) => Some(v.iter()),
            _ => None,
        }
    }

    pub fn directions(&self) -> Option<Vec<&Instruction>> {
        match self {
            Self::Single(i) => Some(vec![i]),
            Self::Multi(v) => Some(v.iter().collect()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Quantity {
    Number(f64),
    String(String),
}

impl Default for Quantity {
    fn default() -> Self {
        Self::String(String::from("N/A"))
    }
}

impl Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(x) => Display::fmt(x, f),
            Self::String(s) => Display::fmt(s, f),
        }
    }
}

#[cfg(test)]
impl Quantity {
    pub(crate) fn number(n: impl Into<f64>) -> Self {
        Self::Number(n.into())
    }

    pub(crate) fn string(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Yield {
    Single(Quantity),
    Multi(Vec<Quantity>),
}

impl Display for Yield {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(q) => Display::fmt(q, f),
            Self::Multi(v) => match v.first() {
                Some(q) => Display::fmt(q, f),
                None => Display::fmt(&Quantity::default(), f),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Accessors, Deserialize)]
#[access(get)]
pub struct Recipe {
    name: String,
    description: String,
    #[serde(rename = "cookTime")]
    cook_time: Option<MaybeDuration>,
    #[serde(rename = "prepTime")]
    prep_time: Option<MaybeDuration>,
    #[serde(rename = "totalTime")]
    total_time: Option<MaybeDuration>,
    #[serde(rename = "recipeYield")]
    yields: Option<Yield>,
    #[serde(rename = "recipeIngredient")]
    ingredients: IngredientList,
    #[serde(rename = "recipeInstructions")]
    directions: Option<InstructionList>,
}

#[cfg(test)]
impl Recipe {
    pub(crate) fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        ingredients: IngredientList,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            ingredients,
            cook_time: None,
            prep_time: None,
            total_time: None,
            yields: None,
            directions: None,
        }
    }

    pub(crate) fn with_directions(self, directions: InstructionList) -> Self {
        Self {
            directions: Some(directions),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum GraphEntry {
    Recipe(Box<Recipe>),
    Nonsense {
        #[serde(rename = "@id")]
        id: String,
    },
}

impl GraphEntry {
    pub fn recipe(&self) -> Option<&Recipe> {
        match self {
            Self::Recipe(r) => Some(r.as_ref()),
            Self::Nonsense { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum SchemaItem {
    Recipe(Box<Recipe>),
    Nonsense {
        #[serde(rename = "@context")]
        context: String,
    },
}

impl SchemaItem {
    pub fn recipe(&self) -> Option<&Recipe> {
        match self {
            Self::Recipe(r) => Some(r.as_ref()),
            Self::Nonsense { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum SchemaEntry {
    Graph {
        #[serde(rename = "@graph")]
        graph: Vec<GraphEntry>,
    },
    Single(SchemaItem),
    Multi(Vec<SchemaItem>),
}

impl crate::Extract for SchemaEntry {
    type Output = Recipe;
    type Collection = Vec<Self::Output>;

    fn extract_recipes(&self) -> Self::Collection {
        match self {
            Self::Graph { graph } => graph.iter().filter_map(|e| e.recipe()).cloned().collect(),
            Self::Single(r) => r.recipe().into_iter().cloned().collect(),
            Self::Multi(entries) => entries.iter().filter_map(|e| e.recipe()).cloned().collect(),
        }
    }
}

impl crate::Scrape for SchemaEntry {
    type Output = Self;
    type Collection = Vec<Self::Output>;

    fn scrape_html(html: impl AsRef<str>) -> Self::Collection {
        let html = scraper::Html::parse_document(html.as_ref());
        let try_from_json = |json| SchemaEntry::from_json_str(json).ok();

        // Unwrap is appropriate here as this is a static selector, which we can reasonably expect
        // to parse successfully
        let selector = scraper::Selector::parse(r#"script[type="application/ld+json"]"#).unwrap();

        html.select(&selector)
            .map(|el| el.text().collect::<String>())
            .filter_map(try_from_json)
            .collect::<Vec<_>>()
    }
}

impl SchemaEntry {
    pub fn from_json_bytes(b: impl AsRef<[u8]>) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(b.as_ref())
    }

    pub fn from_json_reader(r: impl std::io::Read) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(r)
    }

    pub fn from_json_str(json: impl AsRef<str>) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json.as_ref())
    }

    pub fn from_json_value(json: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_maybe_duration() {
        assert_eq!(
            MaybeDuration::from_secs(2i8),
            serde_json::from_value(json!("PT2S")).unwrap()
        );

        assert_eq!(
            MaybeDuration(None),
            serde_json::from_value(json!("PTnullH")).unwrap()
        );
    }

    #[test]
    fn test_ingredient_list_single() {
        let data = json!("The first ingredient.\nThe second one.");
        let result = serde_json::from_value(data);
        assert!(result.is_ok());
        assert_eq!(
            IngredientList::single("The first ingredient.\nThe second one."),
            result.unwrap()
        )
    }

    #[test]
    fn test_ingredient_list_multi() {
        let data = json!(["The first ingredient", "The second one"]);
        let result = serde_json::from_value(data);
        assert!(result.is_ok());
        assert_eq!(
            IngredientList::multi(["The first ingredient", "The second one"]),
            result.unwrap()
        );
    }

    #[test]
    fn test_instruction_simple() {
        let data = json!("Do a thing");
        let result = serde_json::from_value(data);
        assert!(result.is_ok());
        assert_eq!(Instruction::simple("Do a thing"), result.unwrap());
    }

    #[test]
    fn test_instruction_structured() {
        let data = json!({"name": "Do a thing", "text": "Do a thing"});
        let result = serde_json::from_value(data);
        assert!(result.is_ok());
        assert_eq!(Instruction::structured("Do a thing"), result.unwrap());
    }

    #[test]
    fn test_instruction_section() {
        let data = json!({"name": "Prep the thing", "itemListElement": [{"name": "Do a thing", "text": "Do a thing"}]});
        let result = serde_json::from_value(data);
        assert!(result.is_ok());
        assert_eq!(
            InstructionSection::new("Prep the thing", [Instruction::structured("Do a thing")]),
            result.unwrap()
        );
    }

    #[test]
    fn test_instruction_list_single() {
        assert_eq!(
            InstructionList::Single(Instruction::simple("Do a thing")),
            serde_json::from_value(json!("Do a thing")).unwrap(),
        );

        assert_eq!(
            InstructionList::Single(Instruction::structured("Do a thing")),
            serde_json::from_value(json!({"name": "Do a thing", "text": "Do a thing"})).unwrap()
        );
    }

    #[test]
    fn test_instruction_list_multi() {
        assert_eq!(
            InstructionList::Multi(vec![
                Instruction::simple("Do a thing"),
                Instruction::simple("Do another thing")
            ]),
            serde_json::from_value(json!(["Do a thing", "Do another thing"])).unwrap(),
        );

        assert_eq!(
            InstructionList::Multi(vec![
                Instruction::structured("Do a thing"),
                Instruction::structured("Do another thing"),
            ]),
            serde_json::from_value(json!([
                    {"name": "Do a thing", "text": "Do a thing"},
                    {"name": "Do another thing", "text": "Do another thing"}
            ]))
            .unwrap()
        );

        assert_eq!(
            InstructionList::Multi(vec![
                Instruction::simple("Do a thing"),
                Instruction::structured("Do another thing")
            ]),
            serde_json::from_value(json!([
                    "Do a thing",
                    {"name": "Do another thing", "text": "Do another thing"}
            ]))
            .unwrap()
        );
    }

    #[test]
    fn test_instruction_list_sections() {
        assert_eq!(
            InstructionList::Sections(vec![
                InstructionSection::new(
                    "Prep the thing",
                    [Instruction::structured("Do the thing")]
                ),
                InstructionSection::new(
                    "Cook the thing",
                    [Instruction::structured("Do the other thing")]
                )
            ]),
            serde_json::from_value(json!([
                {"name": "Prep the thing", "itemListElement": [{"name": "Do the thing", "text": "Do the thing"}]},
                {"name": "Cook the thing", "itemListElement": [{"name": "Do the other thing", "text": "Do the other thing"}]},
            ]))
            .unwrap(),
        );
    }

    #[test]
    fn test_quantity() {
        assert_eq!(
            Quantity::number(2),
            serde_json::from_value(json!(2)).unwrap()
        );

        assert_eq!(
            Quantity::string("2 cups"),
            serde_json::from_value(json!("2 cups")).unwrap()
        );
    }

    #[test]
    fn test_yield() {
        assert_eq!(
            Yield::Single(Quantity::number(2)),
            serde_json::from_value(json!(2)).unwrap()
        );

        assert_eq!(
            Yield::Multi(vec![Quantity::string("2"), Quantity::string("2 cups"),]),
            serde_json::from_value(json!(["2", "2 cups"])).unwrap()
        );
    }

    #[test]
    fn test_recipe() {
        assert_eq!(
            Recipe::new(
                "A recipe",
                "This is a recipe",
                IngredientList::single("An ingredient. Another one")
            ),
            serde_json::from_value(
                json!({"name": "A recipe", "description": "This is a recipe", "recipeIngredient": "An ingredient. Another one" })
            ).unwrap()
        );

        assert_eq!(
            Recipe::new(
                "A recipe",
                "This is a recipe",
                IngredientList::single("An ingredient. Another one")
            )
            .with_directions(InstructionList::Single(Instruction::simple(
                "Do a thing. Do another thing"
            ))),
            serde_json::from_value(json!({
                    "name": "A recipe",
                    "description": "This is a recipe",
                    "recipeIngredient": "An ingredient. Another one",
                    "recipeInstructions": "Do a thing. Do another thing"
            }))
            .unwrap()
        );
    }
}
