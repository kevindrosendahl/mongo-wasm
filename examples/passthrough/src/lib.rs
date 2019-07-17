use mongo_wasm::prelude::*;

#[derive(Default)]
struct PassthroughPipelineStage;

impl PipelineStage for PassthroughPipelineStage {
    fn get_next(&mut self, doc: Option<Document>) -> GetNextResult {
        match doc {
            Some(doc) => GetNextResult::DocumentReady(doc),
            None => GetNextResult::EOF,
        }
    }
}

mongo_pipeline_stage!(PassthroughPipelineStage);
