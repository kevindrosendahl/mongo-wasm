use mongo_wasm::prelude::*;

struct StatsPipelineStage {
    document_source: Box<dyn DocumentSource>,
    complete: bool,
}

impl PipelineStage for StatsPipelineStage {
    fn new(document_source: Box<dyn DocumentSource>) -> Self {
        StatsPipelineStage {
            document_source,
            complete: false,
        }
    }

    fn next(&mut self) -> Option<Document> {
        if self.complete {
            return None;
        }

        let mut count = 0;
        for _document in self.document_source.iter() {
            count += 1;
        }
        self.complete = true;

        let mut result = Document::new();
        result.insert("count", count);
        Some(result)
    }
}

mongo_pipeline_stage!(StatsPipelineStage);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pipeline() {
        let mut pipeline_stage = mock_mongo_pipeline_stage!(StatsPipelineStage);

        let result = pipeline_stage.next();
        assert!(result.is_some());

        let mut expected = Document::new();
        expected.insert("count", 0);
        assert_eq!(expected, result.unwrap());

        assert!(pipeline_stage.next().is_none());
    }

    #[test]
    fn test_single_doc_pipeline() {
        let mut first = Document::new();
        first.insert("hello", "world");
        let mut pipeline_stage = mock_mongo_pipeline_stage!(StatsPipelineStage, first.clone());

        let result = pipeline_stage.next();
        assert!(result.is_some());

        let mut expected = Document::new();
        expected.insert("count", 1);
        assert_eq!(expected, result.unwrap());

        assert!(pipeline_stage.next().is_none());
    }
}
