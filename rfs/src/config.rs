use crate::{
    fungi::{meta::Tag, Reader, Result, Writer},
    store::{self, Store},
};

pub async fn tag_list(reader: Reader) -> Result<()> {
    let tags = reader.tags().await?;
    if !tags.is_empty() {
        println!("tags:");
    }
    for (key, value) in tags {
        println!("\t{}={}", key, value);
    }
    Ok(())
}

pub async fn tag_add(writer: Writer, tags: Vec<(String, String)>) -> Result<()> {
    for (key, value) in tags {
        writer.tag(Tag::Custom(key.as_str()), value).await?;
    }
    Ok(())
}

pub async fn tag_delete(writer: Writer, keys: Vec<String>, all: bool) -> Result<()> {
    if all {
        writer.delete_tags().await?;
        return Ok(());
    }
    for key in keys {
        writer.delete_tag(Tag::Custom(key.as_str())).await?;
    }
    Ok(())
}

pub async fn store_list(reader: Reader) -> Result<()> {
    let routes = reader.routes().await?;
    if !routes.is_empty() {
        println!("routes:")
    }
    for route in routes {
        println!(
            "\trange:[{}-{}] store:{}",
            route.start, route.end, route.url
        );
    }
    Ok(())
}

pub async fn store_add(writer: Writer, stores: Vec<String>) -> Result<()> {
    let store = store::parse_router(stores.as_slice()).await?;
    for route in store.routes() {
        writer
            .route(
                route.start.unwrap_or(u8::MIN),
                route.end.unwrap_or(u8::MAX),
                route.url,
            )
            .await?;
    }
    Ok(())
}

pub async fn store_delete(writer: Writer, stores: Vec<String>, all: bool) -> Result<()> {
    if all {
        writer.delete_routes().await?;
        return Ok(());
    }
    for store in stores {
        writer.delete_route(store).await?;
    }
    Ok(())
}
