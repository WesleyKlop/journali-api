use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::items::page::Page;
use crate::items::text_field::TextField;
use crate::items::todo::Todo;
use crate::items::todo_item::TodoItem;
use crate::items::{Items, ViewItem};
use crate::schema::items;
use crate::users::user::User;

use super::crud2::raw_crud::Find;
use super::reex_diesel::*;
use super::{ItemLike, ItemType};

#[derive(
    Identifiable, Associations, Insertable, Queryable, Copy, Clone, Serialize,
)]
#[belongs_to(User, foreign_key = "owner_id")]
pub struct Item {
    pub(crate) id: Uuid,
    pub(crate) item_type: ItemType,
    pub(crate) parent_id: Option<Uuid>,
    pub(crate) parent_type: Option<ItemType>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) owner_id: Uuid,
    pub(crate) due_date: Option<DateTime<Utc>>,
}

impl ItemLike for Item {
    fn id(&self) -> Uuid {
        self.id
    }

    fn item_type(&self) -> ItemType {
        self.item_type
    }

    fn parent_id(&self) -> Option<Uuid> {
        self.parent_id
    }

    fn parent_type(&self) -> Option<ItemType> {
        self.parent_type
    }

    fn as_item(&self) -> Item {
        *self
    }
}

impl Default for Item {
    fn default() -> Self {
        Item {
            id: Uuid::default(),
            item_type: 0,
            parent_id: None,
            parent_type: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            owner_id: Uuid::default(),
            due_date: None,
        }
    }
}

impl Item {
    pub fn has_owner<T: super::TypeMarker>(
        id: Uuid,
        owner: Uuid,
        conn: &PgConnection,
    ) -> bool {
        use diesel::dsl::{exists, select};

        select(exists(
            items::table
                .filter(items::owner_id.eq(owner))
                .filter(items::id.eq(id))
                .filter(items::item_type.eq(T::TYPE as i16)),
        ))
        .get_result(conn)
        .unwrap_or(false)
    }

    pub(super) fn delete<T>(id: Uuid, conn: &PgConnection) -> QueryResult<()>
    where
        T: super::TypeMarker,
    {
        diesel::delete(
            items::table
                .filter(items::columns::id.eq(id))
                .filter(items::item_type.eq(T::TYPE as i16)),
        )
        .get_result::<Item>(conn)
        .map(drop)
    }

    pub(super) fn create(&self, conn: &PgConnection) -> QueryResult<Self> {
        diesel::insert_into(items::table).values(self).get_result(conn)
    }

    pub(super) fn update(
        id: &Uuid,
        form: &UpdateItemRequest,
        conn: &PgConnection,
    ) -> QueryResult<Self> {
        diesel::update(items::table.filter(items::id.eq(id)))
            .set(form)
            .get_result(conn)
    }

    pub(super) fn find(
        pid: &Option<Uuid>,
        user: User,
        conn: &PgConnection,
    ) -> QueryResult<Vec<ViewItem>> {
        let mut query =
            items::table.into_boxed().filter(items::owner_id.eq(user.id));
        if pid.is_some() {
            query = query.filter(items::parent_id.eq(pid.unwrap()));
        }
        query.load::<Item>(conn).map(|items| {
            items
                .into_iter()
                .map(|item| match item.item_type {
                    200 => ViewItem::make(
                        item,
                        Items::Todo(
                            Todo::find(item.id, &conn)
                                .expect("Failed to load todo"),
                        ),
                    ),
                    210 => ViewItem::make(
                        item,
                        Items::TodoItem(
                            TodoItem::find(item.id, &conn)
                                .expect("Failed to load todo item"),
                        ),
                    ),
                    100 => ViewItem::make(
                        item,
                        Items::Page(
                            Page::find(item.id, &conn)
                                .expect("Failed to load todo item"),
                        ),
                    ),
                    300 => ViewItem::make(
                        item,
                        Items::TextField(
                            TextField::find(item.id, &conn)
                                .expect("Failed to load todo item"),
                        ),
                    ),
                    _ => unreachable!("Please report an error"),
                })
                .rev()
                .collect()
        })
    }
}

#[derive(AsChangeset, Deserialize)]
#[table_name = "items"]
pub struct UpdateItemRequest {
    pub(crate) parent_id: Option<Uuid>,
    pub(crate) parent_type: Option<ItemType>,
    pub(crate) due_date: Option<DateTime<Utc>>,
}

impl Item {
    pub fn routes(cfg: &mut actix_web::web::ServiceConfig) {
        cfg.service(routes::update).service(routes::get_items);
    }
}

mod routes {
    use actix_web::{get, patch, web, Error, HttpRequest, HttpResponse};
    use serde::Deserialize;
    use uuid::Uuid;

    use crate::{
        database::exec_on_pool, items::item::UpdateItemRequest,
        utils::responsable::Responsable, DbPool,
    };

    use super::Item;

    #[derive(Deserialize)]
    pub struct ItemsByParentRequest {
        parent_id: Option<Uuid>,
    }

    #[get("/items")]
    pub async fn get_items(
        pool: web::Data<DbPool>,
        req: HttpRequest,
        query: web::Query<ItemsByParentRequest>,
    ) -> Result<HttpResponse, Error> {
        let user = req.extensions().get().cloned().unwrap();

        exec_on_pool(&pool, move |conn| {
            Item::find(&query.parent_id, user, &conn)
        })
        .await
        .map(|item| HttpResponse::Ok().json(item))
        .map_err(|_| HttpResponse::InternalServerError().finish().into())
    }

    #[patch("/items/{id}")]
    pub async fn update(
        pool: web::Data<DbPool>,
        id: web::Path<Uuid>,
        form: web::Json<UpdateItemRequest>,
    ) -> Result<HttpResponse, Error> {
        exec_on_pool(&pool, move |conn| {
            Item::update(&id.into_inner(), &form, &conn)
        })
        .await
        .into_response()
    }
}
