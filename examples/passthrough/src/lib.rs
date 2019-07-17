use mongo_wasm::prelude::*;

#[derive(Default)]
struct PassthroughPipelineStage;

impl PipelineStage for PassthroughPipelineStage {
    fn get_next(&mut self, doc: Option<Document>) -> GetNextResult {
        doc.map_or(GetNextResult::EOF, |doc| GetNextResult::DocumentReady(doc))
    }
}

mongo_pipeline_stage!(PassthroughPipelineStage);
