#![deny(warnings)]
use std::str::FromStr;

use sled_extensions::{json::JsonEncoding, Config, DbExt};
use std::collections::HashMap;
use uuid::Uuid;
use warp::http::Uri;

use warp::Filter;
type Db = sled_extensions::structured::Tree<String, JsonEncoding>;

#[tokio::main]
async fn main() {
    let db = Config::default().temporary(true).open().unwrap();
    let tree = db.open_json_tree::<String>("json-tree").unwrap();
    let db = warp::any().map(move || tree.clone());

    let all_binds = warp::path!("bin")
        .and(warp::post())
        .and(warp::body::form())
        .and(db.clone())
        .map(|simple_map: HashMap<String, String>, db: Db| {
            let id = Uuid::new_v4();
            let val = simple_map.get("val").cloned().unwrap_or_default();

            let _ = db.insert(id.as_bytes(), val).unwrap();
            let uri = Uri::from_str(&format!("/bin/{}", id)).unwrap();
            warp::redirect(uri)
        });

    let bin = warp::path!("bin")
        .and(warp::post())
        .and(warp::body::form())
        .and(db.clone())
        .map(|simple_map: HashMap<String, String>, db: Db| {
            let id = Uuid::new_v4();
            let val = simple_map.get("val").cloned().unwrap_or_default();

            let _ = db.insert(id.as_bytes(), val).unwrap();
            let uri = Uri::from_str(&format!("/bin/{}", id)).unwrap();
            warp::redirect(uri)
        });

    let get_bin = warp::path!("bin" / Uuid)
        .and(warp::get())
        .and(db.clone())
        .map(|id: Uuid, db: Db| {
            if let Some(val) = db.get(id.as_bytes()).unwrap() {
                val
            } else {
                "not found".into()
            }
        });

    let home = warp::path::end()
        .and(warp::get())
        .and(db.clone())
        .map(|db: Db| warp::reply::html(get_html(db)));
    let routes = get_bin.or(bin).or(home);

    // Start up the server...
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}

fn get_html(db: Db) -> String {
    r#"
    <html>
<head>
<title> Pastebin
</title>
<link href="https://unpkg.com/tailwindcss@^2/dist/tailwind.min.css" rel="stylesheet">

</head>
<body>
    <form method="POST" action="/bin">
        <textarea class="shadow" name="val">
        </textarea>

        <input type="submit" value="save"/>
    </form>
</body>
</html>
    "#
    .into()
}
