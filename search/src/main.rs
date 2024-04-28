use puffin_index::Index;
use puffin_query::QueryNode;

fn index_and_search() {
    let dir_path = "../../sumup/merchants";

    let mut index = Index::new("disk");
    log::info!("indexing");
    index.index(dir_path);
    log::info!("indexing done");

    log::info!("flushing");
    index.flush().expect("flush index");
    log::info!("flushing done");

    log::info!("searching");
    let result = index.search(QueryNode::new("merchant"));
    log::info!("searching done");

    for f in result {
        println!("match in: {:?}", f.filename);
    }
}

fn search_only() {
    let dir_path = "../../sumup/merchants";

    let mut index = Index::new("disk");
    log::info!("indexing");
    index.index(dir_path);
    log::info!("indexing done");

    // log::info!("searching");
    // let result = index.search(QueryNode::new("lang:go"));
    // for f in result {
    //     println!("match in: {:?}", f.filename);
    // }
    // log::info!("searching done");

    log::info!("searching");
    let result = index.search(QueryNode::new("merchant"));
    for f in result {
        println!("match in: {:?}", f.filename);
    }
    log::info!("searching done");

    log::info!("searching");
    let result = index.search(QueryNode::new("merchant AND business"));
    for f in result {
        println!("match in: {:?}", f.filename);
    }
    log::info!("searching done");

    log::info!("searching");
    let result = index.search(QueryNode::new("profile OR business"));
    for f in result {
        println!("match in: {:?}", f.filename);
    }
    log::info!("searching done");

    log::info!("searching");
    let result = index.search(QueryNode::new("NOT merchant"));
    for f in result {
        println!("match in: {:?}", f.filename);
    }
    log::info!("searching done");
}

fn main() {
    env_logger::init();
    // index_and_search();
    search_only();
}
