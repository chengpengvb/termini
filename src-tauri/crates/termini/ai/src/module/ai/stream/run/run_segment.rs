use crate::module::ai::dto::AiRunContext;
use crate::module::ai::stream::stream_runner::{AiStreamRunner, OpenSegment};

impl AiStreamRunner {
    pub(in crate::module::ai::stream) async fn append_text_delta(&self, context: &AiRunContext, text_segment: &mut Option<OpenSegment>, reasoning_segment: &mut Option<OpenSegment>, delta: &str) {
        self.finish_reasoning_segment(context, reasoning_segment).await;
        if text_segment.is_none() {
            *text_segment = Some(OpenSegment::open());
        }
        let Some(seg) = text_segment.as_mut() else {
            return;
        };
        self.handle_text(context, seg.id.clone(), delta).await;
        seg.content.push_str(delta);
    }

    pub(in crate::module::ai::stream) async fn append_reasoning_delta(&self, context: &AiRunContext, text_segment: &mut Option<OpenSegment>, reasoning_segment: &mut Option<OpenSegment>, delta: &str) {
        self.finish_text_segment(context, text_segment).await;
        if reasoning_segment.is_none() {
            *reasoning_segment = Some(OpenSegment::open());
        }
        let Some(seg) = reasoning_segment.as_mut() else {
            return;
        };
        self.handle_reasoning(context, seg.id.clone(), delta).await;
        seg.content.push_str(delta);
    }

    async fn finish_text_segment(&self, context: &AiRunContext, segment: &mut Option<OpenSegment>) {
        let Some(seg) = segment.take() else {
            return;
        };
        self.handle_text_final(context, seg).await;
    }

    async fn finish_reasoning_segment(&self, context: &AiRunContext, segment: &mut Option<OpenSegment>) {
        let Some(seg) = segment.take() else {
            return;
        };
        self.handle_reasoning_final(context, seg).await;
    }

    pub(in crate::module::ai::stream) async fn finish_open_segments(&self, context: &AiRunContext, text_segment: &mut Option<OpenSegment>, reasoning_segment: &mut Option<OpenSegment>) {
        self.finish_reasoning_segment(context, reasoning_segment).await;
        self.finish_text_segment(context, text_segment).await;
    }
}
