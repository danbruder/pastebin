use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sled_extensions::{json::JsonEncoding, Config, DbExt};
use std::collections::HashMap;
use std::convert::Infallible;
use uuid::Uuid;
use warp::Filter;
use warp::{
    http::{uri::InvalidUri, StatusCode, Uri},
    reject::{Reject, Rejection},
    Reply,
};

type Tree = sled_extensions::structured::Tree<Paste, JsonEncoding>;

#[derive(Clone)]
struct Db {
    tree: Tree,
}

impl Db {
    pub fn get(&self, id: Uuid) -> Result<Paste> {
        match self.tree.get(id.as_bytes()) {
            Ok(Some(val)) => Ok(val),
            _ => panic!("atd"),
        }
    }

    pub fn insert(&self, id: &Uuid, value: &Paste) -> Result<()> {
        match self.tree.insert(id.as_bytes(), value.clone()) {
            Ok(Some(_)) => Ok(()),
            _ => panic!("atd"),
        }
    }

    pub fn all(&self) -> Vec<Paste> {
        self.tree
            .iter()
            .filter_map(std::result::Result::ok)
            .map(|(_, v)| v)
            .collect::<Vec<Paste>>()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("sled db error")]
    SledError(#[from] sled_extensions::Error),
    #[error("Invalid Uri")]
    UriError(#[from] InvalidUri),
    #[error("Not Found")]
    NotFound,
}

type Result<T> = std::result::Result<T, Error>;

impl Reject for Error {}
impl From<Error> for Rejection {
    fn from(other: Error) -> Self {
        warp::reject::custom::<Error>(other.into())
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Paste {
    id: Uuid,
    body: String,
}

impl Paste {
    pub fn uri(&self) -> Uri {
        Uri::from_str(&format!("/bin/{}", self.id)).expect("Cannot fail")
    }
}

#[tokio::main]
async fn main() {
    let db = Config::default().path("./data").open().unwrap();
    let tree = db.open_json_tree::<Paste>("paste").unwrap();
    let tree = Db { tree };
    let db = warp::any().map(move || tree.clone());

    let create_paste = warp::path!("pastes" / "new")
        .and(warp::post())
        .and(warp::body::form())
        .and(db.clone())
        .and_then(|simple_map: HashMap<String, String>, db: Db| async move {
            let id = Uuid::new_v4();
            let body = simple_map.get("body").cloned().unwrap_or_default();
            let paste = Paste { id, body };
            db.insert(&id, &paste)?;
            Ok::<_, Rejection>(warp::redirect(paste.uri()))
        });

    let show_paste = warp::path!("pastes" / Uuid)
        .and(warp::get())
        .and(db.clone())
        .and_then(|id: Uuid, db: Db| async move {
            let paste = db.get(id)?;
            Ok::<_, Rejection>(warp::reply::html(paste.body))
        });

    let home = warp::path::end()
        .and(warp::get())
        .and(db.clone())
        .map(|db: Db| warp::reply::html(get_html(db)));

    let routes = create_paste
        .or(show_paste)
        .or(home)
        .recover(handle_rejection);

    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}

fn get_html(db: Db) -> String {
    let bins = db
        .all()
        .into_iter()
        .map(|p| {
            format!(
                r#"
            <li><a href="/pastes/{0}">{0}</a></li>
            "#,
                p.id
            )
        })
        .collect::<Vec<String>>()
        .join("");

    format!(
        r#"
    <html>
<head>
<title> Pastebin
</title>
    <link href="https://unpkg.com/tailwindcss@^2/dist/tailwind.min.css" rel="stylesheet">

</head>
<body>
    <h1>Pastebin!</h1>
    <form method="POST" action="/pastes/new">
        <div>
            <textarea class="shadow" name="body"/></textarea>
        </div>

        <div>
            <input type="submit" value="Save"/>
        </div>
    </form>
    <div>
        <ul>
            {}
        </ul>
    </div>
</body>
</html>
    "#,
        bins
    )
}

async fn handle_rejection(r: Rejection) -> std::result::Result<impl Reply, Infallible> {
    if let Some(e) = r.find::<Error>() {
        Ok(warp::reply::with_status(
            e.to_string(),
            StatusCode::BAD_REQUEST,
        ))
    } else {
        Ok(warp::reply::with_status(
            String::from("Something bad happened"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}
