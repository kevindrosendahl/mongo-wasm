use mongo_wasm::prelude::*;

#[derive(Default)]
struct ProjectPipelineStage;

impl PipelineStage for ProjectPipelineStage {
    fn get_next(&mut self, doc: Option<bson::Document>) -> GetNextResult {
        doc.map_or(GetNextResult::EOF(None), |mut doc| {
            doc.remove("foo");
            GetNextResult::DocumentReady(doc)
        })
    }
}

mongo_pipeline_stage!(ProjectPipelineStage);
