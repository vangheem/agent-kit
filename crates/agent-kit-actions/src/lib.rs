mod flights;
mod markets;
mod shop;
mod util;
mod web;

use std::sync::Arc;

use agent_kit_core::action::{Action, ActionContext, ActionDescriptor, ActionError, ActionOutput};
use async_trait::async_trait;
use serde_json::Value;

pub fn register() -> Vec<Arc<dyn Action>> {
    vec![
        Arc::new(Kind::WebSearch),
        Arc::new(Kind::WebFetch),
        Arc::new(Kind::ShopSearch),
        Arc::new(Kind::ShopQuote),
        Arc::new(Kind::ShopCheckout),
        Arc::new(Kind::FlightsSearch),
        Arc::new(Kind::FlightsQuote),
        Arc::new(Kind::FlightsBook),
        Arc::new(Kind::MarketsQuote),
        Arc::new(Kind::MarketsCandles),
        Arc::new(Kind::MarketsOrder),
    ]
}

enum Kind {
    WebSearch,
    WebFetch,
    ShopSearch,
    ShopQuote,
    ShopCheckout,
    FlightsSearch,
    FlightsQuote,
    FlightsBook,
    MarketsQuote,
    MarketsCandles,
    MarketsOrder,
}

#[async_trait]
impl Action for Kind {
    fn descriptor(&self) -> ActionDescriptor {
        match self {
            Self::WebSearch => web::search_descriptor(),
            Self::WebFetch => web::fetch_descriptor(),
            Self::ShopSearch => shop::search_descriptor(),
            Self::ShopQuote => shop::quote_descriptor(),
            Self::ShopCheckout => shop::checkout_descriptor(),
            Self::FlightsSearch => flights::search_descriptor(),
            Self::FlightsQuote => flights::quote_descriptor(),
            Self::FlightsBook => flights::book_descriptor(),
            Self::MarketsQuote => markets::quote_descriptor(),
            Self::MarketsCandles => markets::candles_descriptor(),
            Self::MarketsOrder => markets::order_descriptor(),
        }
    }

    async fn execute(
        &self,
        ctx: &ActionContext,
        input: Value,
    ) -> Result<ActionOutput, ActionError> {
        match self {
            Self::WebSearch => web::search(ctx, input).await,
            Self::WebFetch => web::fetch(ctx, input).await,
            Self::ShopSearch => shop::search(ctx, input).await,
            Self::ShopQuote => shop::quote(ctx, input).await,
            Self::ShopCheckout => shop::checkout(ctx, input).await,
            Self::FlightsSearch => flights::search(ctx, input).await,
            Self::FlightsQuote => flights::quote(ctx, input).await,
            Self::FlightsBook => flights::book(ctx, input).await,
            Self::MarketsQuote => markets::quote(ctx, input).await,
            Self::MarketsCandles => markets::candles(ctx, input).await,
            Self::MarketsOrder => markets::order(ctx, input).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use agent_kit_core::action::{
        Action, ActionContext, ActionError, ServiceDirectory, ServiceKind,
    };
    use agent_kit_core::blob::{BlobError, BlobMeta, BlobObject, BlobStore, PutBlob};
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use uuid::Uuid;

    use super::register;

    struct NoServices;

    #[async_trait]
    impl ServiceDirectory for NoServices {
        async fn connection(
            &self,
            _user_id: Uuid,
            _kind: ServiceKind,
        ) -> Result<Option<agent_kit_core::action::ServiceConnection>, ActionError> {
            Ok(None)
        }
    }

    struct UnusedBlobs;

    #[async_trait]
    impl BlobStore for UnusedBlobs {
        async fn put(&self, _request: PutBlob) -> Result<BlobMeta, BlobError> {
            panic!("blob put should not be called");
        }

        async fn get(&self, _id: Uuid) -> Result<BlobObject, BlobError> {
            panic!("blob get should not be called");
        }

        async fn delete(&self, _id: Uuid) -> Result<(), BlobError> {
            panic!("blob delete should not be called");
        }
    }

    fn ctx() -> ActionContext {
        ActionContext {
            user_id: Uuid::nil(),
            mcp_client_id: Uuid::nil(),
            allow_mock: true,
            services: Arc::new(NoServices),
            blobs: Arc::new(UnusedBlobs),
        }
    }

    fn action(id: &str) -> Arc<dyn Action> {
        register()
            .into_iter()
            .find(|item| item.descriptor().id == id)
            .unwrap_or_else(|| panic!("missing {id}"))
    }

    async fn search(query: &str) -> Value {
        let output = action("web.search")
            .execute(&ctx(), json!({ "query": query, "limit": 2 }))
            .await
            .unwrap_or_else(|err| panic!("web.search failed: {err}"));
        output.body
    }

    #[tokio::test]
    async fn mock_web_search_is_deterministic() {
        let first = search("rust agents").await;
        let second = search("rust agents").await;
        let other = search("other query").await;
        assert_eq!(first, second);
        assert_ne!(first, other);
        let results = first["results"].as_array().expect("results");
        assert_eq!(results.len(), 2);
        assert!(
            results[0]["title"]
                .as_str()
                .unwrap()
                .contains("rust agents")
        );
        assert!(results[0]["url"].as_str().unwrap().starts_with("https://"));
        assert!(
            results[0]["snippet"]
                .as_str()
                .unwrap()
                .contains("rust agents")
        );
    }

    #[tokio::test]
    async fn private_url_is_rejected_before_request() {
        let fetch = action("web.fetch");
        let context = ctx();
        for url in [
            "http://127.0.0.1/",
            "http://10.1.2.3/secret",
            "http://192.168.0.8/",
            "http://172.16.5.5/",
            "http://169.254.169.254/latest/meta-data",
            "http://0.0.0.0/",
            "http://224.0.0.1/",
            "http://[::1]/",
            "http://[fd00::1]/",
            "http://[fe80::1]/",
            "http://[::ffff:10.0.0.1]/",
            "http://localhost/admin",
            "file:///etc/passwd",
            "ftp://192.168.0.1/",
        ] {
            let err = fetch
                .execute(&context, json!({ "url": url }))
                .await
                .expect_err(url);
            match err {
                ActionError::InvalidInput { .. } => {}
                other => panic!("{url} was not rejected before a request: {other}"),
            }
        }
    }
}
