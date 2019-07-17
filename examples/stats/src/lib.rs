use mongo_wasm::prelude::*;

use chrono::{DateTime, Utc};
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
    frequencies: Frequencies<i64>,
    online_stats: OnlineStats,
    num_users: u64,
    oldest_user: Option<User>,
    now: DateTime<Utc>,
}

impl StatsPipelineStage {
    fn add_user(&mut self, user: User) {
        if user.birthday >= self.now {
            return;
        }

        self.num_users += 1;
        let age = (self.now.date() - user.birthday.date()).num_weeks() / 52;
        self.frequencies.add(age);
        self.online_stats.add(age);

        match &self.oldest_user {
            None => self.oldest_user = Some(user),
            Some(current_oldest) => {
                if user.birthday < current_oldest.birthday {
                    self.oldest_user = Some(user);
                }
            }
        };
    }

    fn summarize(&self) -> StatsResult {
        let oldest_user_email_address = match &self.oldest_user {
            None => "n/a".to_owned(),
            Some(user) => user.email_address.clone(),
        };

        StatsResult {
            num_users: self.num_users as f64,
            oldest_user_email_address,
            age_stats: AgeStats {
                cardinality: self.frequencies.cardinality() as f64,
                mean: self.online_stats.mean(),
                std_dev: self.online_stats.stddev(),
                variance: self.online_stats.variance(),
            },
        }
    }
}

impl Default for StatsPipelineStage {
    fn default() -> Self {
        StatsPipelineStage {
            frequencies: Frequencies::new(),
            online_stats: OnlineStats::new(),
            num_users: 0,
            oldest_user: None,
            now: Utc::now(),
        }
    }
}

impl PipelineStage for StatsPipelineStage {
    fn get_next(&mut self, doc: Option<Document>) -> GetNextResult {
        match doc {
            // If we haven't received the last doc yet, attempt to deserialize the User and
            // add it to our statistics, then request the next document.
            Some(doc) => {
                let user = bson::from_bson(bson::Bson::Document(doc));
                if user.is_err() {
                    return GetNextResult::NeedsNextDocument;
                }

                self.add_user(user.unwrap());
                GetNextResult::NeedsNextDocument
            }
            // If there are no more documents to process, summarize the statistics and return
            // the summarized document.
            None => {
                let results = self.summarize();
                let results = bson::to_bson(&results).expect("unable to serialize result");
                let results = match results {
                    bson::Bson::Document(doc) => doc,
                    _ => panic!("result did not serialize into a document"),
                };

                GetNextResult::EOF(Some(results))
            }
        }
    }
}

mongo_pipeline_stage!(StatsPipelineStage);
