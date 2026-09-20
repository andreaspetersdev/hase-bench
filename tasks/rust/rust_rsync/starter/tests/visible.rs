use std::ffi::OsString;
use std::path::PathBuf;

use rust_rsync::{Endpoint, PathSpec, parse_invocation, run};

#[test]
fn distinguishes_local_shell_and_daemon_endpoints() {
    let local = parse_invocation(["-a", "C:\\source", "D:\\destination"]).unwrap();
    assert_eq!(
        local.sources,
        vec![Endpoint::Local(PathSpec {
            path: PathBuf::from("C:\\source"),
            copy_contents: false
        })]
    );
    assert_eq!(
        local.destination,
        Endpoint::Local(PathSpec {
            path: PathBuf::from("D:\\destination"),
            copy_contents: false
        })
    );

    let shell = parse_invocation(["source", "host:/destination"]).unwrap();
    assert_eq!(
        shell.destination,
        Endpoint::RemoteShell {
            host: OsString::from("host"),
            path: PathSpec {
                path: PathBuf::from("/destination"),
                copy_contents: false
            }
        }
    );

    let daemon = parse_invocation(["source", "host::module/path"]).unwrap();
    assert_eq!(
        daemon.destination,
        Endpoint::DaemonShell {
            host: OsString::from("host"),
            module_path: PathSpec {
                path: PathBuf::from("module/path"),
                copy_contents: false
            }
        }
    );

    let contents = parse_invocation(["source/", "destination/"]).unwrap();
    assert!(matches!(
        contents.sources[0],
        Endpoint::Local(PathSpec {
            copy_contents: true,
            ..
        })
    ));
}

#[test]
fn performs_an_archive_transfer() {
    let invocation = parse_invocation(["-a", "source/", "destination/"]).unwrap();
    assert_eq!(run(invocation), Ok(()));
}
