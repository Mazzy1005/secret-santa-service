use tide::Request;
use tide::prelude::*;

use std::{
    collections::HashMap,
    fs::File,
    sync::{Arc, Mutex},
};


#[derive(serde::Serialize, serde::Deserialize)]
struct DataBase {
    users: HashMap<String, u32>,
    groups: HashMap<String, Group>,
    id: u32,
}


#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Group{
    users: HashMap<String, u32>,
    admins: HashMap<String, u32>,
    open: bool,
    santas: HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct GroupRequest{
    username: String,
    groupname: String,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let database = match File::open("data.base") {
        Ok(file) => serde_json::from_reader(file).map_err(|err| {
            let err = std::io::Error::from(err);
            std::io::Error::new(
                err.kind(),
                format!("Failed to read from database file. {err}"),
            )
        })?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("Database file not found. Creating one");

            let file = File::create("data.base").map_err(|err| {
                std::io::Error::new(
                    err.kind(),
                    format!("Failed to create database file. {err}"),
                )
             })?;
            let database = DataBase {
                users: HashMap::new(),
		groups: HashMap::new(),
                id: 0,
            };
            serde_json::to_writer(file, &database).map_err(|err| {
                let err = std::io::Error::from(err);
                std::io::Error::new(
                    err.kind(),
                    format!("Failed to write to database file. {err}"),
                )
            })?;
            database
        }
        Err(err) => {
            panic!("Failed to open database file. {err}");
        }
    };
    let state = Arc::new(Mutex::new(database));

    let mut app = tide::with_state(state);

    //app.at("/").get(move |_| async move {Ok(serde_json::json!({"message":hello }))});
    app.at("/add-user").put(add);
    app.at("/add-group").put(|mut request: Request<Arc<Mutex<DataBase>>>| async move {
    let GroupRequest {username ,groupname} = request.body_json().await?;

    let state = request.state();
    let mut guard = state.lock().unwrap();
    
    eprintln!("Adding group {groupname}");
    match guard.users.get(&username) {
        Some(id) => {
            let mut g=Group{
                 users: HashMap::new(),
                 admins: HashMap::new(),
                 open: true,
                 santas: HashMap::new(),
                };
            g.users.insert(username.clone(),*id);
            g.admins.insert(username.clone(),*id);
            guard.groups.insert(groupname,g);
          }, 
          None => todo!(),   }
    Ok(tide::StatusCode::Ok)
});
app.at("/add-user-in-group").put(|mut request: Request<Arc<Mutex<DataBase>>>| async move {
    let GroupRequest {username ,groupname} = request.body_json().await?;

    let state = request.state();
    let mut flag=false;
    let mut guard = state.lock().unwrap();
    if guard.users.contains_key(&username) {
    let id=guard.users[&username];
    if let Some(x) = guard.groups.get_mut(&groupname) {
        if x.open {
            x.users.insert(username,id);
         }
        else {
             flag=true;
         }
       }    
    }
    if !flag {
        Ok(tide::StatusCode::Ok) 
    }
    else {
        Ok(tide::StatusCode::NotAcceptable) 
    }
});
app.at("/delete-group").put(|mut request: Request<Arc<Mutex<DataBase>>>| async move {
    let name: String = request.body_json().await?;

    let state = request.state();
    let mut guard = state.lock().unwrap();
    
    match guard.groups.remove(&name) {
                Some(..) => Ok(serde_json::json!({ "Delete group with id": name })),
                None => Err(tide::Error::from_str(
                    tide::StatusCode::NotFound,
                    format!("Group {name} not found"),
                )),
            }
});
    app.at("/get-user")
            .get(|mut request: Request<Arc<Mutex<DataBase>>>| async move {
            let name: String = request.body_json().await?;

            let state = request.state();
            let guard = state.lock().unwrap();

            eprintln!("Searching for user {name}");

            match guard.users.get(&name) {
                None => Err(tide::Error::from_str(
                    tide::StatusCode::NotFound,
                    format!("User {name} not found"),
                )),
                Some(id) => Ok(serde_json::json!({ "id": id })),
            }
        });

    app.at("/delete-user")
        .put(|mut request: Request<Arc<Mutex<DataBase>>>| async move {
            let name: String = request.body_json().await?;

            let state = request.state();
            let mut guard = state.lock().unwrap();

            match guard.users.remove(&name) {
                Some(name) => Ok(serde_json::json!({ "Delete user with id": name })),
                None => Err(tide::Error::from_str(
                    tide::StatusCode::NotFound,
                    format!("User {name} not found"),
                )),
            }
        });
    app.at("/make-santas")
        .put(|mut request: Request<Arc<Mutex<DataBase>>>| async move {
            let GroupRequest {username ,groupname} = request.body_json().await?;

            let state = request.state();
            let mut guard = state.lock().unwrap();
            if let Some(x) = guard.groups.get_mut(&groupname) {
                if x.open {
                    if x.admins.contains_key(&username) {
                        x.open=false;
                        if x.users.len()>1 {
                            
                            let mut vec: Vec<String>=x.users.clone().into_keys().collect();
                            let first=vec[0].clone();
                            let last=vec[vec.len()-1].clone();
                            x.santas.insert(first,last);
                            while vec.len() > 1 {
                                let first=vec.pop().unwrap();
				let last=vec[vec.len()-1].clone();
                                x.santas.insert(first,last);
                                }
                            }
                        }
                 }
               }
        Ok(tide::StatusCode::Ok)
        });
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}

async fn add(mut request: Request<Arc<Mutex<DataBase>>>) -> tide::Result {
    let name = request.body_json().await?;

    let state = request.state();
    let mut guard = state.lock().unwrap();
    let id=guard.id;
    eprintln!("Adding user {name} with id {id}");
    match guard.users.get(&name) {
        None => { guard.users.insert(name,id);
                   guard.id+=1; },
            Some(..) => {}
         }
    Ok(tide::StatusCode::Ok.into())
}