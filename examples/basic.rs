//! This example demonstrates the schema validation capabilities;
//! as such, you should run it with:
//! ```
//! cargo run --features schema_validation --example basic
//! ```

extern crate avocado;
#[macro_use]
extern crate magnet_derive;
extern crate magnet_schema;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate bson;
extern crate mongodb;

use std::fmt;
use std::fs::create_dir_all;
use std::env::temp_dir;
use std::process::{ Command, Stdio };
use std::error::Error;
use avocado::prelude::*;

// Types for representing a user.

#[derive(Debug, Serialize, Deserialize, BsonSchema)]
#[serde(tag = "type", content = "value")]
enum Contact {
    Phone(String),
    Email(String),
}

#[derive(Debug, Serialize, Deserialize, BsonSchema)]
struct NaiveDate {
    year: u32,
    month: u32,
    day: u32,
}

#[derive(Debug, Serialize, Deserialize, BsonSchema)]
struct User {
    #[serde(rename = "_id")]
    id: ObjectId,
    legal_name: String,
    contact: Option<Contact>,
    birthday: NaiveDate,
}

impl Doc for User {
    type Id = ObjectId;
    const NAME: &'static str = "User";
}

// Types for querying the database for users.

#[derive(Debug, Clone, Copy)]
struct UsersBornBetween {
    min_year: u32,
    max_year: u32,
    has_contact: bool,
}

impl Query<User> for UsersBornBetween {
    type Output = User;

    fn filter(&self) -> Document {
        doc!{
            "$and": [
                { "birthday.year": { "$gte": self.min_year } },
                { "birthday.year": { "$lte": self.max_year } },
                { "contact": { "$exists": true } },
                {
                    "contact": {
                        "$type": if self.has_contact {
                            BsonType::DOCUMENT
                        } else {
                            BsonType::NULL
                        }
                    }
                },
            ]
        }
    }
}

// Putting it all together.

#[derive(Debug)]
struct AnyError(Box<Error>); // fast and loose, don't do this in prod

impl fmt::Display for AnyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<E: Error + 'static> From<E> for AnyError {
    fn from(error: E) -> Self {
        AnyError(Box::new(error))
    }
}

fn example_main() -> Result<(), AnyError> {
    // Spawn a MongoDB instance. Will shut down when this program exits.
    let port = "12984"; // chosen by fair dice roll, guaranteed to be random and unused
    let mut dbpath = temp_dir();

    dbpath.push("avocado_example_db");
    create_dir_all(&dbpath)?;

    let mut mongodb_process = Command::new("mongod")
        .arg("--noscripting")
        .arg("--dbpath")
        .arg(&dbpath)
        .arg("--port")
        .arg(port)
        .stdout(Stdio::piped())
        .spawn()?;

    // the process handle doesn't implement `Drop`, so we need to make sure
    // that the mongo daemon is shut down before this function returns.
    let _guard = {
        struct Guard<F: FnOnce() -> ()>(Option<F>);

        impl<F: FnOnce() -> ()> Guard<F> {
            fn new(f: F) -> Self {
                Guard(Some(f))
            }
        }

        impl<F: FnOnce() -> ()> Drop for Guard<F> {
            fn drop(&mut self) {
                self.0.take().unwrap()();
            }
        }

        Guard::new(|| { mongodb_process.kill().ok(); })
    };

    // Connect to the mongodb server.
    let client = Client::with_uri(&format!("mongodb://localhost:{}/", port))?;
    let db = client.db("avocado_example_db");

    // Create the User collection with BSON schema validation.
    let users: Collection<User> = db.empty_collection()?;

    // Insert some documents into it.
    let user_docs = [
        User {
            id: ObjectId::new()?,
            legal_name: String::from("Donald Ervin Knuth"),
            contact: None, // Don doesn't use email
            birthday: NaiveDate {
                year: 1938,
                month: 1,
                day: 10,
            }
        },
        User {
            id: ObjectId::new()?,
            legal_name: String::from("Steven Paul Jobs"),
            contact: Some(Contact::Email(String::from("sjobs@apple.com"))),
            birthday: NaiveDate {
                year: 1955,
                month: 2,
                day: 24,
            }
        },
    ];

    users.insert_many(&user_docs)?;

    // Query the documents. First, let's see who was born between 1950 and 1960
    // and has specified contact info.
    let born_between_50_and_60 = UsersBornBetween {
        min_year: 1950,
        max_year: 1960,
        has_contact: true,
    };

    println!("");
    println!("Born between 1950 and 1960, provided contact:");
    println!("---------------------------------------------");

    for user in users.find_many(&born_between_50_and_60)? {
        println!("{:#?}", user?);
    }

    // Now let's see if there is anyone from before 1950 without contact info.
    let born_before_1950 = UsersBornBetween {
        min_year: 0,
        max_year: 1950,
        has_contact: false,
    };

    println!("");
    println!("Born before 1950, has no contact:");
    println!("---------------------------------");

    for user in users.find_many(&born_before_1950)? {
        println!("{:#?}", user?);
    }

    Ok(())
}

fn main() {
    example_main().expect("error running example");
}
