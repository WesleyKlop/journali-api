//! This module contains all item definitions used by the application
//!
//! The several items used are listed below:
//! - [`Item`](item/struct.Item.html)
//! - [`Page`](page/struct.Page.html)

use actix_web::{Error, HttpResponse};
use serde::Serialize;
use uuid::Uuid;

use item::Item;

/// Reexport commonly used diesel
/// namespaces
mod reex_diesel {
    pub use diesel::{pg::PgConnection, prelude::*, QueryResult};
}

pub mod item;
pub mod page;
pub mod text_field;
pub mod todo;
pub mod todo_item;

pub type ItemType = i16;

pub trait ItemLike {
    fn id(&self) -> Uuid;
    fn item_type(&self) -> ItemType;
    fn parent_id(&self) -> Option<Uuid>;
    fn parent_type(&self) -> Option<ItemType>;

    fn as_item(&self) -> Item {
        Item {
            id: self.id(),
            item_type: self.item_type(),
            parent_id: self.parent_id(),
            parent_type: self.parent_type(),
            ..Default::default()
        }
    }

    fn as_new_item(&self) -> Item {
        Item { id: Uuid::new_v4(), ..self.as_item() }
    }
}

#[repr(i16)]
pub enum ItemTypeNames {
    Page = 100,
    Todo = 200,
    TodoItem = 210,
    TextField = 300,
}

trait Responsable {
    fn into_response(self) -> Result<HttpResponse, Error>;
}

impl<T, E> Responsable for Result<T, E>
where
    T: Serialize,
{
    fn into_response(self) -> Result<HttpResponse, Error> {
        self.map(|item| HttpResponse::Ok().json(item))
            .map_err(|_| HttResponse::InternalServerError().finish().into())
    }
}
