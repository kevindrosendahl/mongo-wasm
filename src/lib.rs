pub mod prelude {
    pub use super::{
        mongo_pipeline_stage,
        wasm::{debug_msg, encode_return_doc, get_input_doc, ReturnDocument},
        GetNextResult, PipelineStage,
    };

    pub use bson;
    pub use bson::Document;
}

#[derive(Debug)]
pub enum GetNextResult {
    DocumentReady(bson::Document),
    NeedsNextDocument,
    EOF(Option<bson::Document>),
}

/// Represents a aggregation pipeline stage, which can use its DocumentSource
/// to retrieve documents from the prior stage, and returns documents via next().
/// Must be implemented by the user.
pub trait PipelineStage: Default {
    fn get_next(&mut self, doc: Option<bson::Document>) -> GetNextResult;
}

/// Utilities for creating WASM target pipeline stages.
#[doc(hidden)]
pub mod wasm {

    use bson::{decode_document, encode_document, Document};
    use serde::{Deserialize, Serialize};
    use std::ffi::CString;
    use std::mem;
    use std::slice;

    #[no_mangle]
    extern "C" {
        fn print(ptr: i32, len: i32);
    }

    pub fn debug_msg(msg: String) {
        let bytes = msg.into_bytes();
        let len = bytes.len();
        let c_str = CString::new(bytes).unwrap();
        unsafe { print(c_str.as_ptr() as i32, len as i32) };
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct InputDocument {
        pub doc: Option<bson::Document>,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct ReturnDocument {
        pub is_eof: bool,
        pub next_doc: Option<bson::Document>,
    }

    pub fn get_input_doc(ptr: *const u8) -> Option<Document> {
        let mut buf = unsafe {
            // Read length from bson document.
            let len = *(ptr as *const i32) as usize;
            slice::from_raw_parts(ptr, len)
        };

        // Convert it into a bson::Document.
        let input_document = decode_document(&mut buf).expect("failed to decode input document");
        let input_document: InputDocument = bson::from_bson(bson::Bson::Document(input_document))
            .expect("failed to deserialize input document");

        input_document.doc

        // TODO: free the input buffer?
    }

    pub fn encode_return_doc(doc: Document) -> i32 {
        let mut buf = Vec::new();
        encode_document(&mut buf, &doc).expect("failed to encode result document");

        let ptr = buf.as_ptr() as i32;
        mem::forget(buf);
        ptr
    }

    #[macro_export]
    macro_rules! mongo_pipeline_stage {
        ( $ty:ty ) => {
            fn __assert_valid_pipeline()
            where
                $ty: PipelineStage,
            {
                // This error means that your supplied type does not implement mongodb_wasm::PipelineStage.
            }

		    thread_local!(static PIPELINE_STAGE: std::cell::RefCell<$ty> = std::cell::RefCell::new(<$ty>::default()));

            #[no_mangle]
            unsafe extern "C" fn getNext(ptr: i32) -> i32 {
                PIPELINE_STAGE.with(|pipeline_stage| {
                    let doc = get_input_doc(ptr as *const u8);

                    // Call get_next() and convert it to the proper response to bubble back to mongod.
                    let (is_eof, next_doc) = match pipeline_stage.borrow_mut().get_next(doc) {
                        GetNextResult::DocumentReady(doc) => (false, Some(doc)),
                        GetNextResult::NeedsNextDocument  => (false, None),
                        GetNextResult::EOF(maybe_doc) => (true, maybe_doc),
                    };

                    let result = ReturnDocument {
                        is_eof, next_doc
                    };

                    let result_doc = bson::to_bson(&result).expect("unable to serialize result");
                    let result_doc = match result_doc {
                        bson::Bson::Document(doc) => doc,
                        _ => panic!("result did not serialize into a document"),
                    };

                    encode_return_doc(result_doc)
		        })
            }
        };
    }
}
