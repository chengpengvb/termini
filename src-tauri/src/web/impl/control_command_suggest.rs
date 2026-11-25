use bytes::Bytes;
use std::pin::Pin;
use futures::Stream;
use std::task::Poll;
use summer_web::axum;
use std::future::Future;
use summer::plugin::Service;
use async_trait::async_trait;
use summer_web::axum::body::Body;
use crate::base::base_app::BaseApp;
use std::task::Context as TaskContext;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use summer_web::axum::http::{header, HeaderValue};
use summer_web::axum::response::{IntoResponse, Response};
use crate::model::req::req_command_suggest::ReqCommandSuggest;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use crate::service::service_command_suggest::CommandSuggestService;

#[base_app(sort = 912)]
#[derive(Clone, Service)]
pub struct CommandSuggestController {
    #[inject(component)]
    command_suggest_service: CommandSuggestService,
}

#[async_trait]
impl BaseApp for CommandSuggestController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "CommandSuggestController"
    }
}

struct SuggestNdjson<F> {
    work: Option<Pin<Box<F>>>,
    rx: UnboundedReceiver<Vec<String>>,
    finished: Option<ResultModel<Vec<String>>>,
}

impl<F> Stream for SuggestNdjson<F>
where
    F: Future<Output = ResultModel<Vec<String>>>,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(work) = this.work.as_mut() {
            match work.as_mut().poll(cx) {
                Poll::Ready(model) => {
                    this.work = None;
                    this.finished = Some(model);
                }
                Poll::Pending => {}
            }
        }
        if let Poll::Ready(Some(list)) = this.rx.poll_recv(cx) {
            return Poll::Ready(Some(encode_ok(list)));
        }
        if this.work.is_some() {
            return Poll::Pending;
        }
        if let Some(model) = this.finished.take() {
            return Poll::Ready(Some(encode_model(model)));
        }
        Poll::Ready(None)
    }
}

fn encode_ok(list: Vec<String>) -> Result<Bytes, std::io::Error> {
    encode_model(ResultModel::ok(list))
}

fn encode_model(model: ResultModel<Vec<String>>) -> Result<Bytes, std::io::Error> {
    let mut bytes = serde_json::to_vec(&model).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    bytes.push(b'\n');
    Ok(Bytes::from(bytes))
}

#[control_routes(prefix = "/command_suggest")]
impl CommandSuggestController {
    #[summer_web::post("/suggest")]
    async fn suggest(&self, axum::Json(body): axum::Json<ReqCommandSuggest>) -> Response {
        let (tx, rx) = unbounded_channel();
        let service = self.command_suggest_service.clone();
        let stream = SuggestNdjson { rx, finished: None, work: Some(Box::pin(async move { ResultModel::from_result(service.suggest(body, tx).await) })) };
        ([(header::CONTENT_TYPE, HeaderValue::from_static("application/x-ndjson"))], Body::from_stream(stream)).into_response()
    }
}
