//! The web build's [`Storage`](super::Storage): files in the page's
//! IndexedDB database, read into memory before setup and written through.
//!
//! The database is `ponko:<app id>` with one object store, `files`, whose
//! keys are the file names and whose values are the bytes (`Uint8Array`).
//! A write puts the file and its backup in one readwrite transaction, so
//! the browser keeps both or neither.

use std::cell::RefCell;
use std::sync::Mutex;

use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::{Change, MemoryFiles};
use crate::error::{Error, ErrorKind};

const STORE: &str = "files";

/// The files, as last read or written. A game runs on the page's one thread;
/// the lock only makes the static shareable.
static FILES: Mutex<Option<MemoryFiles>> = Mutex::new(None);
/// The database name of the game's app id, set when the game is built.
static NAME: Mutex<Option<String>> = Mutex::new(None);

thread_local! {
    /// The open database, once `preload` opened it.
    static DB: RefCell<Option<web_sys::IdbDatabase>> = const { RefCell::new(None) };
}

fn files<R>(f: impl FnOnce(&mut MemoryFiles) -> R) -> R {
    let mut files = FILES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    f(files.get_or_insert_with(MemoryFiles::default))
}

/// Records the app id whose database [`preload`] opens.
pub(crate) fn open(app_id: &str) {
    *NAME.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(format!("ponko:{app_id}"));
}

pub(crate) fn is_persistent() -> bool {
    DB.with(|db| db.borrow().is_some())
}

pub(crate) fn read(name: &str) -> Option<Vec<u8>> {
    files(|f| f.get(name))
}

pub(crate) fn names() -> Vec<String> {
    files(|f| f.names())
}

pub(crate) fn write(name: &str, bytes: &[u8]) -> Result<(), Error> {
    let changes = files(|f| f.write(name, bytes));
    store(name, changes)
}

pub(crate) fn delete(name: &str) -> Result<(), Error> {
    let changes = files(|f| f.delete(name));
    store(name, changes)
}

/// Stores `changes` in one transaction. Without a database (it didn't
/// open) they stay in memory only.
fn store(name: &str, changes: Vec<Change>) -> Result<(), Error> {
    DB.with(|db| {
        let db = db.borrow();
        let Some(db) = db.as_ref() else {
            return Ok(());
        };
        let fail = |e: JsValue| Error::new(ErrorKind::Io, format!("browser storage: {}", describe(&e))).with_path(name);
        let tx = db
            .transaction_with_str_and_mode(STORE, web_sys::IdbTransactionMode::Readwrite)
            .map_err(fail)?;
        let objects = tx.object_store(STORE).map_err(fail)?;
        for change in changes {
            match change {
                Change::Put(key, bytes) => {
                    let value = js_sys::Uint8Array::from(bytes.as_slice());
                    objects.put_with_key(&value, &JsValue::from_str(&key)).map_err(fail)?;
                }
                Change::Remove(key) => {
                    objects.delete(&JsValue::from_str(&key)).map_err(fail)?;
                }
            }
        }
        // A transaction that fails later (the quota) is aborted whole: the
        // stored file stays the old one. Say so in the console.
        let name = name.to_string();
        let report = wasm_bindgen::closure::Closure::once_into_js(move |_: web_sys::Event| {
            log_error(&format!("ponko: browser storage could not keep {name:?}"));
        });
        tx.set_onabort(Some(report.unchecked_ref()));
        Ok(())
    })
}

/// Opens the game's database and reads every file into memory. Runs before
/// setup on the web; does nothing for a game without an app id.
pub(crate) async fn preload() {
    let Some(name) = NAME.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone() else {
        return;
    };
    match load(&name).await {
        Ok(db) => DB.with(|cell| *cell.borrow_mut() = Some(db)),
        Err(e) => log_error(&format!(
            "ponko: browser storage {name:?} is unavailable, files last only until the page closes: {}",
            describe(&e)
        )),
    }
}

async fn load(name: &str) -> Result<web_sys::IdbDatabase, JsValue> {
    let factory = web_sys::window()
        .ok_or("no window")?
        .indexed_db()?
        .ok_or("no IndexedDB")?;
    let open = factory.open_with_u32(name, 1)?;
    let request = open.clone();
    let upgrade = wasm_bindgen::closure::Closure::once_into_js(move |_: web_sys::Event| {
        if let Ok(db) = request.result() {
            let db: web_sys::IdbDatabase = db.unchecked_into();
            if let Err(e) = db.create_object_store(STORE) {
                log_error(&format!("ponko: could not create browser storage: {}", describe(&e)));
            }
        }
    });
    open.set_onupgradeneeded(Some(upgrade.unchecked_ref()));
    done(&open).await?;
    let db: web_sys::IdbDatabase = open.result()?.unchecked_into();

    let tx = db.transaction_with_str(STORE)?;
    let objects = tx.object_store(STORE)?;
    // Both lists come in key order, so they pair up.
    let keys = objects.get_all_keys()?;
    let values = objects.get_all()?;
    done(&keys).await?;
    done(&values).await?;
    let keys: js_sys::Array = keys.result()?.unchecked_into();
    let values: js_sys::Array = values.result()?.unchecked_into();
    files(|f| {
        for (key, value) in keys.iter().zip(values.iter()) {
            if let (Some(key), Ok(bytes)) = (key.as_string(), value.dyn_into::<js_sys::Uint8Array>()) {
                f.insert(key, bytes.to_vec());
            }
        }
    });
    Ok(db)
}

/// Waits for `request` to succeed or fail.
async fn done(request: &web_sys::IdbRequest) -> Result<(), JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        request.set_onsuccess(Some(&resolve));
        request.set_onerror(Some(&reject));
    });
    JsFuture::from(promise).await?;
    Ok(())
}

fn describe(e: &JsValue) -> String {
    e.as_string()
        .or_else(|| e.dyn_ref::<js_sys::Error>().map(|e| String::from(e.message())))
        .unwrap_or_else(|| format!("{e:?}"))
}

fn log_error(message: &str) {
    web_sys::console::error_1(&JsValue::from_str(message));
}
