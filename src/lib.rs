mod schema_org;
pub use schema_org::{Recipe as SchemaOrgRecipe, SchemaEntry as SchemaOrgEntry};

pub trait Extract {
    type Output;
    type Collection: IntoIterator<Item = Self::Output>;

    fn extract_recipes(&self) -> Self::Collection;
}

pub trait Scrape {
    type Output;
    type Collection: IntoIterator<Item = Self::Output>;

    fn scrape_html(html: impl AsRef<str>) -> Self::Collection;
}
