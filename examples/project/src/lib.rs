use mongo_wasm::prelude::*;

struct ProjectPipelineStage {
    iter: Box<dyn Iterator<Item = Document>>,
}

impl PipelineStage for ProjectPipelineStage {
    fn new(document_source: Box<dyn DocumentSource>) -> Self {
        ProjectPipelineStage {
            iter: document_source.iter(),
        }
    }

    fn next(&mut self) -> Option<Document> {
        self.iter.next().map(|mut doc| {
            doc.remove("foo");
            doc
        })
    }
}

mongo_pipeline_stage!(ProjectPipelineStage);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pipeline() {
        let mut pipeline_stage = mock_mongo_pipeline_stage!(ProjectPipelineStage);
        let result = pipeline_stage.next();
        assert!(result.is_none());
    }

    #[test]
    fn test_single_doc_pipeline() {
        let mut doc = Document::new();
        doc.insert("hello", "world");
        doc.insert("foo", "bar");
        let mut pipeline_stage = mock_mongo_pipeline_stage!(ProjectPipelineStage, doc);

        let result = pipeline_stage.next();
        assert!(result.is_some());

        let mut expected = Document::new();
        expected.insert("hello", "world");
        assert_eq!(expected, result.unwrap());

        let result = pipeline_stage.next();
        assert!(result.is_none());
    }
}
