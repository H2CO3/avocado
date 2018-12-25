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
#[macro_use]
extern crate bson;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mongodb;
extern crate avocado;

use std::env::temp_dir;
use std::fs::create_dir_all;
use std::sync::Mutex;
use std::collections::HashSet;
use std::process::{ Command, Child, Stdio };
use avocado::error::Result;
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

implement_tests!{
    #[test]
    fn foo() {
    }

    #[test]
    fn bar() -> Result<()> {
        Ok(())
    }
}
