//! Integration tests for checking high-level functionality of the most
//! important moving parts. Namely, these tests exercise the following modules:
//! * [`db`](db/index.html)
//! * [`coll`](coll/index.html)
//! * [`cursor`](cursor/index.html)
//! * [`doc`](doc/index.html)
//! * [`ops`](ops/index.html)

#[macro_use]
extern crate scopeguard;
#[macro_use]
extern crate lazy_static;
extern crate bson;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mongodb;
#[macro_use]
extern crate magnet_derive;
extern crate magnet_schema;
#[macro_use]
extern crate avocado_derive;
extern crate avocado;

use std::env::temp_dir;
use std::fs::create_dir_all;
use std::sync::Mutex;
use std::collections::{ HashSet, BTreeSet };
use std::process::{ Command, Child, Stdio };
use avocado::error::{ Error, Result };
use avocado::prelude::*;

/// Used for killing the MongoDB server process once all tests have run.
struct ProcessGuard {
    handle: Child,
    owners: HashSet<&'static str>,
}

impl ProcessGuard {
    fn new(handle: Child, owners: &[&'static str]) -> Self {
        ProcessGuard {
            handle: handle,
            owners: owners.iter().map(|&item| item).collect(),
        }
    }

    fn resign(&mut self, owner: &str) {
        let pid = self.handle.id();

        println!("=== ProcessGuard(#{}): Resigning owner '{}'", pid, owner);
        self.owners.remove(owner);

        if self.owners.is_empty() {
            println!("=== ProcessGuard(#{}): All owners resigned; killing", pid);
            self.handle.kill().expect("couldn't kill child process");
        }
    }
}

macro_rules! implement_tests {
    // TODO(H2CO3): use `?` Kleene operator instead of `*` once Rust 1.32 is out
    ($(#[test] $(#[$attr:meta])* fn $test_name:ident() $(-> $ret_ty:ty)* $test_code:block)*) => {
        lazy_static! {
            static ref DB_SERVER_GUARD: Mutex<ProcessGuard> = {
                let dbpath = {
                    let mut tmp = temp_dir();
                    tmp.push(DB_NAME);
                    create_dir_all(&tmp).expect("couldn't create DB temp dir");
                    tmp
                };
                let owners = [$(stringify!($test_name),)*];
                let process = Command::new("mongod")
                    .arg("--noscripting")
                    .arg("--dbpath")
                    .arg(&dbpath)
                    .arg("--port")
                    .arg(DB_PORT)
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("couldn't start DB server; do you have Mongo installed?");

                Mutex::new(ProcessGuard::new(process, &owners))
            };
        }

        $(
            #[test]
            $(#[$attr])*
            /// TODO(H2CO3): use `?` Kleene operator instead of `*` once Rust 1.32 is out
            fn $test_name() $(-> $ret_ty)* {
                defer!({
                    DB_SERVER_GUARD.lock().unwrap().resign(stringify!($test_name));
                });
                $test_code
            }
        )*
    }
}

/// Not Quite Random (distinct from the one in `examples/basic.rs`)
static DB_PORT: &str = "12985";
/// Similar but distinct DB (directory) name, for the same reason.
static DB_NAME: &str = "avocado_test_db";

lazy_static! {
    /// We don't care that the client is not RAII-destroyed. Its resources (eg.
    /// memory, pipe/socket/file descriptor) will be cleaned up by the OS.
    /// The important thing is that the server process is shut down so we don't
    /// spam the process space with useless servers (which would also expose
    /// whomever is running the test suite to a needless security risk.)
    static ref DB_HANDLE: Database = {
        Client::with_uri(
            &format!("mongodb://localhost:{}/", DB_PORT)
        ).expect(
            "can't connect to mongod server"
        ).db(
            DB_NAME
        )
    };
}

// A couple of distinct types for making collections with.

#[derive(Debug, Clone, Serialize, Deserialize, BsonSchema, Doc)]
#[index(
    name = "URL",
    unique,
    keys(url = "ascending"),
)]
struct Repo {
    _id: Uid<Repo>,
    owner: Uid<User>,
    name: String,
    url: String,
    vcs: Vcs,
    issues: Vec<Issue>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, BsonSchema)]
enum Vcs {
    Git,
    Svn,
    Hg,
}

#[derive(Debug, Clone, Serialize, Deserialize, BsonSchema, Doc)]
#[id_type = "u64"]
struct Issue {
    #[serde(rename = "_id")]
    number: Uid<Issue>,
    description: String,
    opened: Uid<User>,
    assignee: Option<Uid<User>>,
    resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, BsonSchema, Doc)]
#[index(
    name = "username",
    unique = true,
    keys(username = "ascending")
)]
struct User {
    _id: Uid<User>,
    legal_name: String,
    username: String,
    repos: HashSet<Uid<Repo>>,
    groups: HashSet<Uid<Group>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BsonSchema, Doc)]
struct Group {
    _id: Uid<Group>,
    name: String,
    description: String,
}

// Finally, the actual tests.

implement_tests!{
    #[test]
    fn basic_insertion_deletion_raw_doc() -> Result<()> {
        let coll: Collection<Group> = DB_HANDLE.empty_collection()?;

        let group_1 = Group {
            _id: Uid::new_oid()?,
            name: String::from("Fancy FinTech, Inc."),
            description: String::from("IoT AI on the quantum blockchain"),
        };
        let group_2 = Group {
            _id: Uid::new_oid()?,
            name: String::from("PHP Shop, Ltd."),
            description: String::from("It's Shit But They Pay For It [TM]"),
        };

        // No documents before insertion
        assert!(coll.find_one(doc!{})?.is_none());
        assert!(!coll.find_many(doc!{})?.has_next()?);
        assert_eq!(coll.count(doc!{})?, 0);

        // Can insert but don't allow duplicates
        let id_1 = coll.insert_one(&group_1)?;
        assert!(coll.insert_many(vec![&group_1]).is_err());
        assert_eq!(id_1, group_1._id);

        let ids_2 = coll.insert_many(vec![&group_2])?;
        assert!(coll.insert_one(&group_2).is_err());
        assert_eq!(ids_2, [group_2._id.clone()]);

        // Can retrieve documents after insertion
        assert_eq!(
            coll.find_one(doc!{ "name": "PHP Shop, Ltd." })?.as_ref(),
            Some(&group_2)
        );
        assert_eq!(
            coll.find_many(doc!{ "_id": &group_1._id })?.collect::<Result<Vec<_>>>()?,
            vec![group_1.clone()]
        );

        // Can delete after insertion, too
        assert!(coll.delete_entity(&group_2)?);
        assert_eq!(coll.delete_entities(vec![&group_1])?, 1);

        assert!(!coll.delete_entity(&group_1)?);
        assert_eq!(coll.delete_entities(vec![&group_2])?, 0);

        // No documents after deletion
        assert!(coll.find_one(doc!{})?.is_none());
        assert!(!coll.find_many(doc!{})?.has_next()?);
        assert_eq!(coll.count(doc!{})?, 0);

        Ok(())
    }

    #[test]
    fn update_query_delete_custom_ops() -> Result<()> {
        use avocado::coll::{ UpdateOneResult, UpsertOneResult };

        let users: Collection<User> = DB_HANDLE.empty_collection()?;
        let repos: Collection<Repo> = DB_HANDLE.empty_collection()?;

        let mut user_1 = User {
            _id: Uid::new_oid()?,
            legal_name: String::from("John Doe"),
            username: String::from("jdoe"),
            repos: HashSet::new(),
            groups: HashSet::new(),
        };
        let impostor = User {
            _id: Uid::new_oid()?,
            legal_name: String::from("Jane Doe"),
            ..user_1.clone() // username is same but should have been different
        };
        let mut user_2 = User {
            _id: Uid::new_oid()?,
            legal_name: String::from("Steven Smith"),
            username: String::from("steve"),
            repos: HashSet::new(),
            groups: HashSet::new(),
        };

        assert_eq!(users.insert_many(vec![&user_1, &user_2])?,
                   vec![user_1._id.clone(), user_2._id.clone()]);

        // unique index should be enforced
        assert!(users.insert_one(&impostor).is_err());

        let mut repo_1 = Repo {
            _id: Uid::new_oid()?,
            owner: user_1._id.clone(),
            name: String::from("frobnicator"),
            url: String::from("githoob.com/jdoe/frobnicator.git"),
            vcs: Vcs::Git, // because why would anyone use anything else
            issues: Vec::new(), // it's perfect
        };
        let repo_2 = Repo {
            _id: Uid::new_oid()?,
            owner: user_2._id.clone(),
            name: String::from("SpaceY"),
            url: String::from("githoob.com/steve/scam.git"),
            vcs: Vcs::Svn, // you should already be suspicious at this point
            issues: Vec::new(),
        };
        assert_eq!(repos.insert_one(&repo_1)?,
                   repo_1._id.clone());

        // replacing existing entity
        repo_1.name = String::from("Gadget");
        assert_eq!(repos.replace_entity(&repo_1)?,
                   UpdateOneResult { matched: true, modified: true });

        // trying to replace nonexistent
        assert_eq!(repos.replace_entity(&repo_2)?,
                   UpdateOneResult { matched: false, modified: false });

        // upserting nonexistent
        assert_eq!(repos.upsert_entity(&repo_2)?,
                   UpsertOneResult { matched: false,
                                     modified: false,
                                     upserted_id: Some(repo_2._id.clone()) });

        // Add the repos to the owners' profile

        #[derive(Debug, Clone)]
        struct UpdateUserRepos<'a> {
            user_id: &'a Uid<User>,
            repos: &'a HashSet<Uid<Repo>>
        }

        impl<'a> Update<User> for UpdateUserRepos<'a> {
            fn filter(&self) -> Document {
                doc!{
                    "_id": self.user_id
                }
            }

            fn update(&self) -> Document {
                doc!{
                    "$set": {
                        "repos": bson::to_bson(self.repos).unwrap_or_default()
                    }
                }
            }
        }

        user_1.repos.insert(repo_1._id.clone());
        user_2.repos.insert(repo_2._id.clone());

        assert_eq!(
            users.update_one(UpdateUserRepos {
                user_id: &user_1._id,
                repos: &user_1.repos,
            })?,
            UpdateOneResult { matched: true, modified: true }
        );
        assert_eq!(
            // with reference too, to test blanket impls
            users.update_one(&UpdateUserRepos {
                user_id: &user_2._id,
                repos: &user_2.repos,
            })?,
            UpdateOneResult { matched: true, modified: true }
        );

        // Query the repos and check which user they belong to
        // (this tests projections)

        #[derive(Debug, Clone)]
        struct UserNameForRepo<'a> {
            repo_id: &'a Uid<Repo>,
        }

        impl<'a> Query<User> for UserNameForRepo<'a> {
            type Output = String;

            fn filter(&self) -> Document {
                doc!{
                    "repos": {
                        "$elemMatch": {
                            "$eq": self.repo_id,
                        }
                    }
                }
            }

            fn transform(mut doc: Document) -> Result<Bson> {
                doc.remove("username").ok_or_else(|| {
                    Error::new("missing key `username` in retrieved document")
                })
            }

            fn options() -> FindOptions {
                FindOptions {
                    projection: Some(doc!{
                        "_id": false,
                        "username": true,
                    }),
                    ..Default::default()
                }
            }
        }

        assert_eq!(
            users.find_one(
                UserNameForRepo { repo_id: &repo_1._id }
            )?,
            Some(user_1.username)
        );
        assert_eq!(
            users.find_one(
                // with reference too, to test blanket impls
                &UserNameForRepo { repo_id: &repo_2._id }
            )?,
            Some(user_2.username)
        );

        Ok(())
    }

    #[test]
    fn advanced_ops() -> Result<()> {
        let issues: Collection<Issue> = DB_HANDLE.empty_collection()?;

        let bug = Issue {
            number: Uid::from_raw(1),
            description: String::from("it's buggered, fix it already"),
            opened: Uid::new_oid()?,
            assignee: None,
            resolved: false,
        };
        let pebkac = Issue {
            number: Uid::from_raw(2),
            description: String::from("it doesn't work"),
            opened: Uid::new_oid()?,
            assignee: Some(Uid::new_oid()?),
            resolved: true, // it's a feature
        };
        let feature_request = Issue {
            number: Uid::from_raw(3),
            description: String::from("why doesn't it also brew coffee"),
            opened: Uid::new_oid()?,
            assignee: None,
            resolved: true,
        };
        let issue_entities = vec![&bug, &pebkac, &feature_request];

        issues.insert_many(issue_entities.clone())?;

        // Testing the `Distinct` trait
        #[derive(Debug, Clone, Copy)]
        struct ResolvedValues;

        impl Distinct<Issue> for ResolvedValues {
            type Output = i64;

            const FIELD: &'static str = "resolved";

            fn transform(raw: Bson) -> Result<Bson> {
                Ok(match raw {
                    Bson::Boolean(b) => Bson::I64(b as _),
                    _ => raw
                })
            }
        }

        let mut bits = issues.distinct(ResolvedValues)?;
        bits.sort();

        let mut bits_ref = issues.distinct(&ResolvedValues)?;
        bits_ref.sort();

        assert_eq!(bits,     &[0, 1]);
        assert_eq!(bits_ref, &[0, 1]);

        // Testing the `Pipeline` trait

        #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
        struct Descriptions {
            assigned:   BTreeSet<String>,
            unassigned: BTreeSet<String>,
        }

        #[derive(Debug, Clone, Copy)]
        struct DescriptionsByStatus;

        impl Pipeline<Issue> for DescriptionsByStatus {
            type Output = Descriptions;

            fn stages(&self) -> Vec<Document> {
                vec![
                    doc!{
                        "$group": {
                            "_id": {
                                "$eq": [ "$assignee", null ]
                            },
                            "descriptions": { "$addToSet": "$description" },
                        }
                    },
                    doc!{
                        "$facet": {
                            "assigned": [
                                { "$match": { "_id": false } },
                            ],
                            "unassigned": [
                                { "$match": { "_id": true } },
                            ],
                        }
                    },
                    doc!{
                        "$project": {
                            "assigned": { "$arrayElemAt": ["$assigned", 0] },
                            "unassigned": { "$arrayElemAt": ["$unassigned", 0] },
                        }
                    },
                    doc!{
                        "$project": {
                            "assigned":   "$assigned.descriptions",
                            "unassigned": "$unassigned.descriptions",
                        }
                    },
                ]
            }
        }

        let descriptions_from_pipeline =
            issues.aggregate(&DescriptionsByStatus)?.next().unwrap()?;

        let descriptions_from_test = {
            let mut d = Descriptions::default();

            for &issue in &issue_entities {
                if issue.assignee.is_some() {
                    d.assigned.insert(issue.description.clone());
                } else {
                    d.unassigned.insert(issue.description.clone());
                }
            }

            d
        };

        assert_eq!(
            descriptions_from_pipeline,
            descriptions_from_test,
        );

        Ok(())
    }

    #[test]
    fn keep_server_alive() {}
}
