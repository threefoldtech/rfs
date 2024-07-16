use crate::{
    fungi::{meta::Tag, Reader, Result, Writer},
    store::Store,
};

/// configure FL with the provided tags/stores and print the result tags/stores
pub async fn config<S: Store>(
    writer: Writer,
    reader: Reader,
    store: S,
    tags: Vec<(String, String)>,
    stores: Vec<String>,
    replace: bool,
) -> Result<()> {
    if !tags.is_empty() && replace {
        writer.delete_tags().await?;
    }
    if !stores.is_empty() && replace {
        writer.delete_routes().await?;
    }
    for (key, value) in tags {
        writer.tag(Tag::Custom(key.as_str()), value).await?;
    }

    for route in store.routes() {
        writer
            .route(
                route.start.unwrap_or(u8::MIN),
                route.end.unwrap_or(u8::MAX),
                route.url,
            )
            .await?;
    }

    let tags = reader.tags().await?;
    if !tags.is_empty() {
        println!("tags:");
    }
    for (key, value) in tags {
        println!("\t{}={}", key, value);
    }

    let routes = reader.routes().await?;
    if !routes.is_empty() {
        println!("routes:")
    }
    for route in routes {
        println!("\trange:[{}-{}] store:{}", route.start, route.end, route.url);
    }

    Ok(())
}
