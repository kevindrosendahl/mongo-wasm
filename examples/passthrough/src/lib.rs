use mongo_wasm::prelude::*;

struct PassthroughPipelineStage {
    iter: Box<dyn Iterator<Item = Document>>,
}

impl PipelineStage for PassthroughPipelineStage {
    fn new(document_source: Box<dyn DocumentSource>) -> Self {
        PassthroughPipelineStage {
            iter: document_source.iter(),
        }
    }

    fn next(&mut self) -> Option<Document> {
        self.iter.next()
    }
}

mongo_pipeline_stage!(PassthroughPipelineStage);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pipeline() {
        let mut pipeline_stage = mock_mongo_pipeline_stage!(PassthroughPipelineStage);
        let result = pipeline_stage.next();
        assert!(result.is_none());
    }

    #[test]
    fn test_single_doc_pipeline() {
        let mut first = Document::new();
        first.insert("hello", "world");
        let mut pipeline_stage =
            mock_mongo_pipeline_stage!(PassthroughPipelineStage, first.clone());

        let result = pipeline_stage.next();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), first);

        let result = pipeline_stage.next();
        assert!(result.is_none());
    }
}
