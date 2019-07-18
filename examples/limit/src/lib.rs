use mongo_wasm::prelude::*;

#[derive(Default)]
struct LimitPipelineStage {
    num_seen: u32,
}

impl PipelineStage for LimitPipelineStage {
    fn get_next(&mut self, doc: Option<Document>) -> GetNextResult {
        match doc {
            None => GetNextResult::EOF(None),
            Some(doc) => {
                self.num_seen += 1;

                if self.num_seen >= 2 {
                    GetNextResult::EOF(Some(doc))
                } else {
                    GetNextResult::DocumentReady(doc)
                }
            }
        }
    }
}

mongo_pipeline_stage!(LimitPipelineStage);
