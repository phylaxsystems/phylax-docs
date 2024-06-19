use crate::{
    activities::{Activity, ActivityContext, FgWorkType},
    events::{Event, EventBuilder, EventType},
    tasks::task::WorkContext,
};
use async_trait::async_trait;
use color_eyre::Result;

use flume::r#async::RecvStream;
use phylax_respond::{Webhook, WebhookClient, WebhookRes};
use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

#[async_trait]
impl<T> Activity for Webhook<T>
where
    T: Send + Sync + WebhookClient + 'static,
{
    type Command = WebhookCmd;
    type InternalStream = RecvStream<'static, Self::InternalStreamItem>;
    type InternalStreamItem = WebhookRes;

    /// During bootstraping, the webhook will make a test request. If the request fails, the whole
    /// bootstrap will fail.
    async fn bootstrap(&mut self, _context: Arc<ActivityContext>) -> Result<()> {
        let (url, req) = self.client.test_request(self.raw_request.clone())?;
        let mut res = self.client.make_request_to_target(req, url).await?;
        // Set the test flag to `true`, signifiying that the execution was a test during the
        // bootstsrap
        res.test = true;
        let return_code = res.res_code;
        // Send the response to the event bus
        self.internal_tx.send(res)?;
        // TODO(odysesas): This should become a Webhook error
        if return_code != 200 {
            let msg = eyre::Report::msg(format!(
                "The webhook's request succeeded with a non 200 error code: {return_code}"
            ));
            return Err(msg);
        }
        Ok(())
    }
    fn decide(
        &self,
        mut events: Vec<&Event>,
        _context: &WorkContext,
    ) -> std::option::Option<Vec<WebhookCmd>> {
        Some(vec![WebhookCmd::CallWebhook(events.pop().unwrap().clone())])
    }

    /// Send the request
    async fn do_work(&mut self, _command: Vec<WebhookCmd>, ctx: WorkContext) -> eyre::Result<()> {
        let context_map = ctx.into_erased_map();
        // Construct Request
        let (url, req) = self.client.target_and_request(self.raw_request.clone(), context_map)?;
        // Send Request
        let res = self.client.make_request_to_target(req, url).await?;
        let return_code = res.res_code;
        self.internal_tx.send(res)?;
        if return_code != 200 {
            let msg = eyre::Report::msg(format!(
                "The webhook's request succeeded with a non-200 error code: {return_code}"
            ));
            return Err(msg);
        }
        Ok(())
    }

    fn work_type(&self) -> FgWorkType {
        FgWorkType::SpawnNonBlocking
    }

    /// The internal stream is the response of the request
    async fn internal_stream(&self) -> Option<Self::InternalStream> {
        Some(self.internal_rx.clone().into_stream())
    }

    /// Convert the request response into an event (of type [`EventType::ActionExecution`]) to be
    /// sent in the event bus
    async fn internal_stream_work(
        &mut self,
        item: Self::InternalStreamItem,
        _ctx: Arc<ActivityContext>,
    ) -> Option<Event> {
        Some(item.into())
    }
}

#[derive(Debug)]
pub enum WebhookCmd {
    CallWebhook(Event),
}

impl Display for WebhookCmd {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            WebhookCmd::CallWebhook(_) => write!(f, "CallWebhook"),
        }
    }
}

impl From<WebhookRes> for Event {
    fn from(res: WebhookRes) -> Self {
        EventBuilder::new()
            .with_type(EventType::ActionExecution)
            .with_body(serde_json::to_value(res).unwrap())
            .build()
    }
}
#[cfg(test)]
mod tests {

    use crate::{
        activities::{user_defined::webhook::WebhookCmd, Activity, FgWorkType},
        events::{EventBuilder, EventType},
        mocks::mock_context::mock_activity_context,
        tasks::task::WorkContext,
    };
    use phylax_config::{GeneralWebhookConfig, SensitiveValue, WebhookAuth};
    use phylax_respond::{GenClient, Webhook};
    use serde_json::json;
    use tokio_stream::StreamExt;

    fn create_generic_webhook(target: impl ToString) -> Webhook<GenClient> {
        let config = GeneralWebhookConfig {
            target: target.to_string(),
            method: "POST".parse().unwrap(),
            payload: "{\"test\": 1 }".to_owned(),
            authentication: SensitiveValue::new(WebhookAuth::None),
        };
        config.try_into().unwrap()
    }

    #[tokio::test]
    async fn test_bootstrap_generic() {
        let mut server = mockito::Server::new_async().await;
        let target = format!("{}/example", server.url());
        let mut webhook = create_generic_webhook(target.clone());
        let mut stream = webhook.internal_stream().await.unwrap();
        let raw_req = webhook.raw_request.clone();
        let mock =
            server.mock("post", "/example").match_body(raw_req.payload.as_str()).expect(1).create();
        let context = mock_activity_context(None);
        webhook.bootstrap(context).await.expect("Failed to bootsrap");
        let res = stream.next().await.unwrap();
        assert_eq!(res.res_code, 200);
        assert!(res.test);
        mock.assert();
    }

    #[tokio::test]
    async fn test_do_work_and_internal_work() {
        let mut server = mockito::Server::new_async().await;
        let target = server.url();
        let activity_context = mock_activity_context(None);
        let context = WorkContext::new(activity_context.clone(), EventBuilder::new().build());
        let mut webhook = create_generic_webhook(target);
        let mut stream = webhook.internal_stream().await.unwrap();
        let event = EventBuilder::new().with_origin(123).build();
        let cmds = vec![WebhookCmd::CallWebhook(event)];
        let body = "{\"status\": \"ok\"}";
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();
        webhook.do_work(cmds, context.clone()).await.unwrap();
        let res = stream.next().await.unwrap();
        assert_eq!(res.res_code, 200);
        assert!(!res.test);
        assert_eq!(&res.body, body);
        let event = webhook.internal_stream_work(res.clone(), activity_context).await.unwrap();
        assert_eq!(event.get_type(), &EventType::ActionExecution);
        assert_eq!(event.get_body(), serde_json::to_value(res).unwrap());
        mock.assert();
    }

    #[tokio::test]
    async fn test_fgwork_type() {
        let webhook = create_generic_webhook("http://example.com");
        let fgwork_type = webhook.work_type();
        assert_eq!(fgwork_type, FgWorkType::SpawnNonBlocking);
    }
    #[tokio::test]
    async fn test_do_work_and_env_subst() {
        let mut server = mockito::Server::new_async().await;
        let target = server.url();
        let body = json! ({"chaos" : "ladder", "noice": vec!["sarah", "marah"]});
        let event = EventBuilder::new().with_origin(123).with_body(body).build();
        let context = WorkContext::new(mock_activity_context(None), event.clone());
        let config = GeneralWebhookConfig {
            target: target.to_string(),
            method: "POST".parse().unwrap(),
            payload: "Chaos is a ${event.body.chaos}, but we have at least ${event.body.noice[0]} and ${event.body.noice[1]}"
                .to_owned(),
            authentication: SensitiveValue::new(WebhookAuth::None),
        };
        let mut webhook: Webhook<GenClient> = config.try_into().unwrap();
        let mut stream = webhook.internal_stream().await.unwrap();
        let cmd = WebhookCmd::CallWebhook(event);
        let cmds = vec![cmd];
        let res_body = "{\"type\": \"ok\"}";
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(res_body)
            .match_body("Chaos is a ladder, but we have at least sarah and marah")
            .create();
        webhook.do_work(cmds, context).await.unwrap();
        let res = stream.next().await.unwrap();
        mock.assert();
        assert_eq!(res.res_code, 200);
        assert!(!res.test);
        assert_eq!(&res.body, res_body);
    }
}
