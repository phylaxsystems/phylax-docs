use crate::error::WebhookError;
use async_trait::async_trait;
use base64::Engine;
use color_eyre::Result;
use phylax_common::subst;
use phylax_config::{ChatWebhookConfig, GeneralWebhookConfig, WebhookAuth};
use phylax_tracing::tracing::debug;
use reqwest::{
    header::{HeaderMap, HeaderValue, IntoHeaderName, AUTHORIZATION},
    Client, Method, Response,
};
use serde::{Deserialize, Serialize};
use serenity::{
    builder::CreateEmbedAuthor,
    http::Http,
    model::{
        channel::Embed,
        prelude::{webhook::Webhook as SerenityWebhook, Message},
    },
};
use slack_morphism::{
    prelude::{SlackClient as MorphismSlackClient, *},
    slack_blocks,
};
use std::{
    collections::HashMap,
    fmt,
    fmt::{Display, Formatter},
};
use url::Url;

pub type SlackClient = MorphismSlackClient<SlackClientHyperHttpsConnector>;
pub type GenClient = Client;
pub type DiscordClient = Http;

#[async_trait]
pub trait WebhookClient {
    type Req: Send + Sync;
    type RawReq: Send + Sync + Clone + Display;

    async fn make_request_to_target(
        &self,
        request: Self::Req,
        target: Option<Url>,
    ) -> Result<WebhookRes, WebhookError>;

    /// Construct a request from a raw request
    fn target_and_request(
        &self,
        raw_req: Self::RawReq,
        event_jq: HashMap<String, String>,
    ) -> Result<(Option<Url>, Self::Req), WebhookError>;

    /// Constract a test request from a raw request. This is used to test the webhook during
    /// the bootstrap phase. User can override this method to customize the test request.
    // TODO(odysseas): Modify the test request to be clearly a test request. Perhaps
    // the user could offer diff values for test (e.g diff payload, diff message if chat, etc.)
    fn test_request(
        &self,
        raw_req: Self::RawReq,
    ) -> Result<(Option<Url>, Self::Req), WebhookError> {
        let test_event = HashMap::new();
        self.target_and_request(raw_req, test_event)
    }
}

#[async_trait]
impl WebhookClient for GenClient {
    type Req = reqwest::Request;
    type RawReq = GenericRequest;

    async fn make_request_to_target(
        &self,
        request: Self::Req,
        _target: Option<Url>,
    ) -> Result<WebhookRes, WebhookError> {
        debug!(?request, "Webhook Request");
        let res: Response = self.execute(request).await?;
        debug!(?res, "Webhook Response");
        Ok(WebhookRes::try_from_http(res).await?)
    }

    fn target_and_request(
        &self,
        raw_req: Self::RawReq,
        event_jq: HashMap<String, String>,
    ) -> Result<(Option<Url>, Self::Req), WebhookError> {
        let payload = subst(&event_jq, &raw_req.payload);
        Ok((
            None,
            self.request(raw_req.method.clone(), raw_req.url.clone())
                .timeout(std::time::Duration::from_millis(500))
                .body(payload)
                .headers(raw_req.headers)
                .build()
                .map_err(|e| WebhookError::WebhookBuildError { source: e.into() })?,
        ))
    }
}
#[async_trait]
impl WebhookClient for SlackClient {
    type Req = SlackApiPostWebhookMessageRequest;
    type RawReq = ChatRequest<Slack>;

    async fn make_request_to_target(
        &self,
        request: Self::Req,
        url: Option<Url>,
    ) -> Result<WebhookRes, WebhookError> {
        debug!(?request, "Sending webhook to Slack");
        let res: SlackApiPostWebhookMessageResponse =
            self.post_webhook_message(&url.unwrap(), &request).await?;
        debug!(?res, "Received response from Slack");
        Ok(WebhookRes::try_from_slack(res)?)
    }

    fn target_and_request(
        &self,
        raw_req: Self::RawReq,
        event_jq: HashMap<String, String>,
    ) -> Result<(Option<Url>, Self::Req), WebhookError> {
        let title = subst(&event_jq, &raw_req.title);
        let description = subst(&event_jq, &raw_req.description);
        let content = SlackMessageContent::new().with_blocks(slack_blocks![
            some_into(SlackHeaderBlock::new(pt!(title))),
            some_into(SlackDividerBlock::new()),
            some_into(SlackSectionBlock::new().with_text(pt!(description)))
        ]);
        Ok((Some(raw_req.platform.target), SlackApiPostWebhookMessageRequest::new(content)))
    }
}

#[async_trait]
impl WebhookClient for DiscordClient {
    type Req = serde_json::Value;
    type RawReq = ChatRequest<Discord>;

    async fn make_request_to_target(
        &self,
        embed: Self::Req,
        url: Option<Url>,
    ) -> Result<WebhookRes, WebhookError> {
        debug!(embed=?embed, "Sending webhook to Discord");
        let webhook = SerenityWebhook::from_url(&self, url.unwrap().as_ref()).await?;
        // The boolean is wether we want the server to wait for the message to be posted
        // before sending a response
        let res = webhook.execute(&self, true, |req| req.embeds(vec![embed])).await?;
        debug!(?res, "Received response from Discord");
        Ok(WebhookRes::try_from_discord(res)?)
    }

    fn target_and_request(
        &self,
        raw_req: Self::RawReq,
        event_jq: HashMap<String, String>,
    ) -> Result<(Option<Url>, Self::Req), WebhookError> {
        let title = subst(&event_jq, &raw_req.title);
        let description = subst(&event_jq, &raw_req.description);
        let embed = Embed::fake(|e| {
            e.title(&title)
                .set_author(
                    CreateEmbedAuthor::default()
                        .name(&raw_req.username)
                        .icon_url(&raw_req.avatar_url)
                        .to_owned(),
                )
                .description(description)
        });
        Ok((Some(raw_req.platform.target), embed))
    }
}

impl<T> Display for Webhook<T>
where
    T: Send + Sync + WebhookClient,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Webhook {}", self.raw_request)
    }
}

#[derive(Debug)]
pub struct Webhook<T>
where
    T: Send + Sync + WebhookClient,
{
    pub client: T,
    pub raw_request: T::RawReq,
    pub internal_rx: flume::Receiver<WebhookRes>,
    pub internal_tx: flume::Sender<WebhookRes>,
}

pub struct WebhookBuilder<T>
where
    T: Send + Sync,
{
    inner: T,
}

impl WebhookBuilder<GenericRequest> {
    fn new() -> Self {
        Self { inner: GenericRequest::default() }
    }

    fn with_method(mut self, method: Method) -> Self {
        self.inner.method = method;
        self
    }

    fn with_target(mut self, target: Url) -> Self {
        self.inner.url = target;
        self
    }

    fn with_payload(mut self, payload: String) -> Self {
        self.inner.payload = payload;
        self
    }

    fn with_headers<T: Into<HeaderValue>, I: IntoHeaderName>(
        mut self,
        header: I,
        value: T,
    ) -> Self {
        self.inner.headers.insert(header, value.into());
        self
    }

    fn with_basic_auth(mut self, username: String, password: String) -> Result<Self, WebhookError> {
        let encoded = base64::engine::general_purpose::STANDARD_NO_PAD
            .encode(format!("{username}:{password}").as_bytes());
        self =
            self.with_headers(AUTHORIZATION, HeaderValue::from_str(&format!("Basic {encoded}"))?);
        Ok(self)
    }

    fn with_bearer_auth(mut self, token: String) -> Result<Self, WebhookError> {
        self = self.with_headers(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {token}"))?);
        Ok(self)
    }

    fn build(self) -> Webhook<GenClient> {
        let client = GenClient::new();
        let (internal_tx, internal_rx) = flume::bounded(50);
        Webhook { client, raw_request: self.inner, internal_rx, internal_tx }
    }
}

impl<T> WebhookBuilder<ChatRequest<T>>
where
    T: Default + Send + Sync,
{
    fn new() -> Self {
        Self { inner: ChatRequest::default() }
    }

    fn with_username(mut self, username: String) -> Self {
        self.inner.username = username;
        self
    }

    fn with_avatar_url(mut self, avatar_url: String) -> Self {
        self.inner.avatar_url = avatar_url;
        self
    }

    fn with_title(mut self, title: String) -> Self {
        self.inner.title = title;
        self
    }

    fn with_description(mut self, description: String) -> Self {
        self.inner.description = description;
        self
    }

    fn with_platform(mut self, platform: T) -> Self {
        self.inner.platform = platform;
        self
    }
}

#[derive(Clone)]
pub struct Slack {
    pub target: Url,
}

impl Slack {
    pub fn new(target: Url) -> Self {
        Self { target }
    }
}

impl Default for Slack {
    fn default() -> Self {
        Self::new(Url::parse("https://slack.com").unwrap())
    }
}

#[derive(Clone)]
pub struct Discord {
    pub target: Url,
}

impl Discord {
    pub fn new(target: Url) -> Self {
        Self { target }
    }
}

impl Default for Discord {
    fn default() -> Self {
        Self::new(Url::parse("https://discord.com/").unwrap())
    }
}

impl WebhookBuilder<ChatRequest<Slack>> {
    fn build(self) -> Webhook<SlackClient> {
        let client = SlackClient::new(SlackClientHyperConnector::new());
        let (internal_tx, internal_rx) = flume::bounded(50);
        Webhook { client, raw_request: self.inner, internal_rx, internal_tx }
    }
}

impl WebhookBuilder<ChatRequest<Discord>> {
    fn build(self) -> Webhook<DiscordClient> {
        // We don't need a token
        let client = DiscordClient::new("");
        let (internal_tx, internal_rx) = flume::bounded(50);
        Webhook { client, raw_request: self.inner, internal_rx, internal_tx }
    }
}

#[derive(Clone, Debug)]
pub struct ChatRequest<T> {
    username: String,
    avatar_url: String,
    title: String,
    description: String,
    platform: T,
}

#[derive(Clone, Debug)]
pub struct GenericRequest {
    pub method: Method,
    pub url: Url,
    pub payload: String,
    pub headers: HeaderMap,
}

impl Default for GenericRequest {
    fn default() -> Self {
        Self {
            method: Method::POST,
            url: Url::parse("https://google.com").unwrap(),
            payload: String::new(),
            headers: HeaderMap::new(),
        }
    }
}

impl<P> Default for ChatRequest<P>
where
    P: Default,
{
    fn default() -> Self {
        Self {
            username: String::new(),
            avatar_url: String::new(),
            title: String::new(),
            description: String::new(),
            platform: P::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebhookRes {
    pub res_code: u16,
    pub body: String,
    pub test: bool,
}

impl WebhookRes {
    async fn try_from_http(res: Response) -> Result<Self, reqwest::Error> {
        let res_code = res.status().as_u16();
        let body = res.text().await?;
        Ok(Self { res_code, body, test: false })
    }

    ///TODO: this doesn't seem right
    fn try_from_slack(_res: SlackApiPostWebhookMessageResponse) -> Result<Self> {
        Ok(Self { res_code: 200, body: "".to_string(), test: false })
    }

    ///TODO: this doesn't seem right
    fn try_from_discord(_res: Option<Message>) -> Result<Self> {
        Ok(Self { res_code: 200, body: "".to_string(), test: false })
    }
}

impl TryFrom<GeneralWebhookConfig> for Webhook<GenClient> {
    type Error = WebhookError;
    fn try_from(config: GeneralWebhookConfig) -> Result<Self, Self::Error> {
        let mut builder = WebhookBuilder::<GenericRequest>::new()
            .with_method(config.method)
            .with_target(Url::parse(&config.target)?)
            .with_payload(config.payload.clone());
        if serde_json::from_str::<serde_json::Value>(&config.payload).is_ok() {
            builder = builder
                .with_headers("Content-Type", HeaderValue::from_str("application/json").unwrap());
        }
        builder = match config.authentication.into_inner() {
            WebhookAuth::Basic { username, password } => {
                builder.with_basic_auth(username, password)?
            }
            WebhookAuth::Bearer(token) => builder.with_bearer_auth(token)?,
            WebhookAuth::None => builder,
        };
        Ok(builder.build())
    }
}

impl TryFrom<ChatWebhookConfig> for Webhook<SlackClient> {
    type Error = WebhookError;
    fn try_from(config: ChatWebhookConfig) -> Result<Self, Self::Error> {
        let webhook = WebhookBuilder::<ChatRequest<Slack>>::new()
            .with_username(config.username)
            .with_avatar_url(config.avatar_url.to_string())
            .with_description(config.description)
            .with_title(config.title)
            .with_platform(Slack::new(Url::parse(config.target.as_ref())?));
        Ok(webhook.build())
    }
}

impl TryFrom<ChatWebhookConfig> for Webhook<DiscordClient> {
    type Error = WebhookError;
    fn try_from(config: ChatWebhookConfig) -> Result<Self, Self::Error> {
        let webhook = WebhookBuilder::<ChatRequest<Discord>>::new()
            .with_username(config.username)
            .with_avatar_url(config.avatar_url.to_string())
            .with_description(config.description)
            .with_title(config.title)
            .with_platform(Discord::new(Url::parse(config.target.as_ref())?));
        Ok(webhook.build())
    }
}

impl Display for GenericRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Generic Request [ {} {} | Headers {:?} | Payload {} ]",
            self.method, self.url, self.headers, self.payload
        )
    }
}

impl Display for ChatRequest<Slack> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Slack [ Title: {} ]", self.title)
    }
}

impl Display for ChatRequest<Discord> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Discord [ Title: {} ]", self.title)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use mockito::Server;

    #[tokio::test]
    async fn test_general_webhook() {
        let mut server = Server::new_async().await;
        let _mock =
            server.mock("POST", "/").with_status(200).with_body("webhook response").create();

        let client = GenClient::new();
        let (internal_tx, internal_rx) = flume::bounded(10);
        let webhook = Webhook {
            client,
            internal_rx,
            internal_tx,
            raw_request: GenericRequest {
                method: Method::POST,
                url: Url::parse(&server.url()).unwrap(),
                payload: "test payload".to_string(),
                headers: HeaderMap::new(),
            },
        };
        let (target, req) = webhook
            .client
            .target_and_request(webhook.raw_request.clone(), Default::default())
            .unwrap();
        let res = webhook.client.make_request_to_target(req, target).await.unwrap();
        assert_eq!(res.res_code, 200);
        assert_eq!(res.body, "webhook response");
    }

    // /// Create a quick server using axum for testing purposes
    // /// This server will be bound to any available port
    // async fn create_quick_server() -> (SocketAddr, Receiver<Value>) {
    //     let (tx, rx) = flume::bounded::<Value>(10);
    //     // setup routes
    //     let tx2 = tx.clone();
    //     let app = Router::new().route(
    //         "/",
    //         axum::routing::post(|request: axum::extract::Request| async move {
    //             let a = tx2.clone();
    //             let body = request.into_body();
    //             let raw = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    //             let val: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    //             a.send(val);
    //         }),
    //     );

    //     // bind to any available port
    //     let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    //     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    //     // run it
    //     let server = axum::serve(listener, app.into_make_service());

    //     tokio::spawn(async move {
    //         if let Err(e) = server.await {
    //             eprintln!("server error: {}", e);
    //         }
    //     });
    //     (addr, rx)
    // }

    // #[tokio::test]
    // async fn test_slack_webhook() {
    //     let (target, body_rx) = create_quick_server().await;
    //     tokio::time::sleep(Duration::from_secs(1)).await;

    //     let (internal_tx, internal_rx) = flume::bounded(10);
    //     let client = SlackClient::new(SlackClientHyperConnector::new());
    //     let webhook = Webhook {
    //         client,
    //         internal_rx,
    //         internal_tx,
    //         raw_request: ChatRequest {
    //             username: "test_user".to_string(),
    //             avatar_url: "http://example.com/avatar.png".to_string(),
    //             title: "test_title".to_string(),
    //             description: "test_description".to_string(),
    //             platform: Slack { target: format!("https://{target}/").parse().unwrap() },
    //         },
    //     };
    //     let (target, req) =
    // webhook.client.target_and_request(webhook.raw_request.clone()).unwrap();     let res =
    // webhook.client.make_request_to_target(req, target).await.unwrap();     assert_eq!(res.
    // res_code, 200);     assert_eq!(res.body, "webhook response");
    // }

    // #[tokio::test]
    // async fn test_discord_webhook() {
    //     let mut server = mockito::Server::new();
    //     let _m = server.mock("POST", "/").with_status(200).with_body("webhook
    // response").create();     let (internal_tx, internal_rx) = flume::bounded(10);
    //     let client = DiscordClient::new("test_token");
    //     let webhook = Webhook {
    //         client,
    //         raw_request: ChatRequest {
    //             username: "test_user".to_string(),
    //             avatar_url: "http://example.com/avatar.png".to_string(),
    //             title: "test_title".to_string(),
    //             description: "test_description".to_string(),
    //             platform: Discord { target: Url::parse(&server.url()).unwrap() },
    //         },
    //         internal_rx,
    //         internal_tx,
    //     };
    //     let (target, request) = webhook.client.target_and_request(webhook.raw_request).unwrap();
    //     let res = webhook.client.make_request_to_target(request, target).await.unwrap();
    //     assert_eq!(res.res_code, 200);
    //     assert_eq!(res.body, "webhook response");
    // }

    #[tokio::test]
    async fn test_webhook_error() {
        let mut server = Server::new_async().await;
        let _m = server.mock("POST", "/").with_status(500).with_body("webhook error").create();
        let client = GenClient::new();
        let (internal_tx, internal_rx) = flume::bounded(10);
        let webhook = Webhook {
            client,
            internal_rx,
            internal_tx,
            raw_request: GenericRequest {
                method: Method::POST,
                url: Url::parse(&server.url()).unwrap(),
                payload: "test payload".to_string(),
                headers: HeaderMap::new(),
            },
        };

        let (_target, req) = webhook
            .client
            .target_and_request(webhook.raw_request.clone(), Default::default())
            .unwrap();
        let res = webhook.client.make_request_to_target(req, None).await.unwrap();
        assert_ne!(res.res_code, 200);
    }

    #[tokio::test]
    async fn test_webhook_builder() {
        let builder = WebhookBuilder::<GenericRequest>::new()
            .with_method(Method::POST)
            .with_target(Url::parse("http://example.com").unwrap())
            .with_payload("test payload".to_string())
            .with_headers("Content-Type", HeaderValue::from_str("application/json").unwrap());

        let webhook = builder.build();
        assert_eq!(webhook.raw_request.method, Method::POST);
        assert_eq!(webhook.raw_request.url, Url::parse("http://example.com").unwrap());
        assert_eq!(webhook.raw_request.payload, "test payload".to_string());
        assert_eq!(webhook.raw_request.headers.get("Content-Type").unwrap(), "application/json");
    }

    #[tokio::test]
    async fn test_webhook_auth() {
        let builder = WebhookBuilder::<GenericRequest>::new()
            .with_method(Method::POST)
            .with_target(Url::parse("http://example.com").unwrap())
            .with_payload("test payload".to_string())
            .with_basic_auth("username".to_string(), "password".to_string())
            .unwrap();

        let webhook = builder.build();
        let auth_header = webhook.raw_request.headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert!(auth_header.starts_with("Basic "));
    }
}
