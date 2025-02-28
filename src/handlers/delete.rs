use crate::handlers::html::{make_error, ErrorResponse};
use crate::{Database, Error, Page};
use axum::extract::{Path, State};
use axum::response::Redirect;
use axum_extra::extract::SignedCookieJar;

pub async fn delete(
    Path(id): Path<String>,
    State(db): State<Database>,
    State(page): State<Page>,
    jar: SignedCookieJar,
) -> Result<Redirect, ErrorResponse> {
    async {
        let id = id.parse()?;
        let uid = db.get_uid(id).await?;
        let can_delete = jar
            .get("uid")
            .map(|cookie| cookie.value().parse::<i64>())
            .transpose()
            .map_err(|err| Error::CookieParsing(err.to_string()))?
            .zip(uid)
            .is_some_and(|(user_uid, db_uid)| user_uid == db_uid);

        if !can_delete {
            Err(Error::Delete)?;
        }

        db.delete(id).await?;

        Ok(Redirect::to("/"))
    }
    .await
    .map_err(|err| make_error(err, page.clone()))
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::Client;
    use reqwest::StatusCode;

    #[tokio::test]
    async fn delete_via_link() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new().await;

        let data = crate::handlers::insert::form::Entry {
            text: "FooBarBaz".to_string(),
            extension: None,
            expires: "0".to_string(),
            password: "".to_string(),
            title: "".to_string(),
        };

        let res = client.post("/").form(&data).send().await?;
        let uid_cookie = res.cookies().find(|cookie| cookie.name() == "uid").unwrap();
        assert_eq!(uid_cookie.name(), "uid");
        assert!(uid_cookie.value().len() > 40);
        assert_eq!(uid_cookie.path(), None);
        assert!(uid_cookie.http_only());
        assert!(uid_cookie.same_site_strict());
        assert!(!uid_cookie.secure());
        assert_eq!(uid_cookie.domain(), None);
        assert_eq!(uid_cookie.expires(), None);
        assert_eq!(uid_cookie.max_age(), None);

        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        let location = res.headers().get("location").unwrap().to_str()?;
        let id = location.replace("/", "");

        let res = client.get(&format!("/delete/{id}")).send().await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        let res = client.get(&format!("/{id}")).send().await?;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
