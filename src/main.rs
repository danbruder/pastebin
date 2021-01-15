use std::str::FromStr;

use serde_json::{json, Value};
use sled_extensions::{json::JsonEncoding, Config, DbExt};
use std::collections::HashMap;
use uuid::Uuid;
use warp::http::Uri;

use warp::Filter;
type Db = sled_extensions::structured::Tree<Value, JsonEncoding>;

#[tokio::main]
async fn main() {
    let db = Config::default().temporary(true).open().unwrap();
    let tree = db.open_json_tree::<Value>("json-tree").unwrap();
    let db = warp::any().map(move || tree.clone());

    let bin = warp::path!("bin")
        .and(warp::post())
        .and(warp::body::form())
        .and(db.clone())
        .map(|simple_map: HashMap<String, String>, db: Db| {
            let id = Uuid::new_v4();
            let val = simple_map.get("val").cloned().unwrap_or_default();
            let val = json!({
                "id": id,
                "val": val
            });

            let _ = db.insert(id.as_bytes(), val).unwrap();
            let uri = Uri::from_str(&format!("/bin/{}", id)).unwrap();
            warp::redirect(uri)
        });

    let get_bin = warp::path!("bin" / Uuid)
        .and(warp::get())
        .and(db.clone())
        .map(|id: Uuid, db: Db| {
            let stuff = if let Some(val) = db.get(id.as_bytes()).unwrap() {
                format!("{}", val["val"].as_str().unwrap())
            } else {
                "not found".into()
            };

            warp::reply::html(stuff)
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
    let bins = db
        .iter()
        .filter_map(Result::ok)
        .map(|(_, v)| {
            let id = v["id"].as_str().unwrap();
            format!(
                r#"
            <li><a href="/bin/{}">{}</a></li>
            "#,
                id, id,
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
    <form method="POST" action="/bin">
        <textarea class="shadow" name="val"/></textarea>

        <input type="submit" value="save"/>
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
