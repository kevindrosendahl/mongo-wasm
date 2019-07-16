use chrono::{offset::TimeZone, DateTime, Utc};
use mongo_wasm::prelude::*;
use serde::{Deserialize, Serialize};
use stats::{Frequencies, OnlineStats};

#[derive(Deserialize, Serialize, Debug)]
struct User {
    pub name: String,
    pub birthday: DateTime<Utc>,
    pub email_address: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
struct StatsResult {
    num_users: f64,
    oldest_user_email_address: String,
    age_stats: AgeStats,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
struct AgeStats {
    cardinality: f64,
    mean: f64,
    std_dev: f64,
    variance: f64,
}

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

        let mut frequencies = Frequencies::new();
        let mut online_stats = OnlineStats::new();
        let mut num_users = 0;
        let mut oldest_user = None;
        let now = Utc::now();

        self.document_source
            .iter()
            .map(|doc| bson::from_bson(bson::Bson::Document(doc)))
            .filter_map(Result::ok)
            .for_each(|user: User| {
                if user.birthday >= now {
                    return;
                }

                num_users += 1;
                let age = (now.date() - user.birthday.date()).num_weeks() / 52;
                frequencies.add(age);
                online_stats.add(age);

                match &oldest_user {
                    None => oldest_user = Some(user),
                    Some(current_oldest) => {
                        if user.birthday < current_oldest.birthday {
                            oldest_user = Some(user);
                        }
                    }
                }
            });

        self.complete = true;
        let oldest_user_email_address = match oldest_user {
            None => "n/a".to_owned(),
            Some(user) => user.email_address,
        };

        let results = StatsResult {
            num_users: num_users as f64,
            oldest_user_email_address,
            age_stats: AgeStats {
                cardinality: frequencies.cardinality() as f64,
                mean: online_stats.mean(),
                std_dev: online_stats.stddev(),
                variance: online_stats.variance(),
            },
        };

        let results = bson::to_bson(&results).expect("unable to serialize result");
        let results = match results {
            bson::Bson::Document(doc) => doc,
            _ => panic!("result did not serialize into a document"),
        };

        Some(results)
    }
}

mongo_pipeline_stage!(StatsPipelineStage);

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! serialize_to_doc_or_fail {
        ( $doc:expr ) => {
            match bson::to_bson(&$doc).expect("unable to serialize result") {
                bson::Bson::Document(doc) => doc,
                _ => panic!("result did not serialize into a document"),
            }
        };
    }

    #[test]
    fn test_empty_pipeline() {
        let mut pipeline_stage = mock_mongo_pipeline_stage!(StatsPipelineStage);

        let result = pipeline_stage.next();
        assert!(result.is_some());

        let result: StatsResult = bson::from_bson(bson::Bson::Document(result.unwrap())).unwrap();
        let expected = StatsResult {
            num_users: 0.0,
            oldest_user_email_address: "n/a".to_owned(),
            age_stats: AgeStats {
                cardinality: 0.0,
                mean: 0.0,
                std_dev: 0.0,
                variance: 0.0,
            },
        };
        assert_eq!(expected, result);

        assert!(pipeline_stage.next().is_none());
    }

    #[test]
    fn test_multiple_doc_pipeline() {
        let young = User {
            name: "Younger Person".to_owned(),
            birthday: Utc.ymd(2000, 1, 2).and_hms(3, 4, 5),
            email_address: "young@mongodb.com".to_owned(),
        };
        let young = serialize_to_doc_or_fail!(young);

        let middle = User {
            name: "Middle Person".to_owned(),
            birthday: Utc.ymd(1975, 1, 2).and_hms(3, 4, 5),
            email_address: "middle@mongodb.com".to_owned(),
        };
        let middle = serialize_to_doc_or_fail!(middle);

        let old = User {
            name: "Older  Person".to_owned(),
            birthday: Utc.ymd(1950, 1, 2).and_hms(3, 4, 5),
            email_address: "old@mongodb.com".to_owned(),
        };
        let old = serialize_to_doc_or_fail!(old);

        let mut pipeline_stage = mock_mongo_pipeline_stage!(StatsPipelineStage, young, middle, old);

        let result = pipeline_stage.next();
        assert!(result.is_some());

        let result: StatsResult = bson::from_bson(bson::Bson::Document(result.unwrap())).unwrap();
        let expected = StatsResult {
            num_users: 3.0,
            oldest_user_email_address: "old@mongodb.com".to_owned(),
            age_stats: AgeStats {
                cardinality: 3.0,
                mean: 44.0,
                std_dev: 20.412414523193153,
                variance: 416.6666666666667,
            },
        };
        assert_eq!(expected, result);

        assert!(pipeline_stage.next().is_none());
    }
}
