mod tools;

use std::collections::HashMap;

use backend::utils::roles::{models::{Role, PrivilegeChangeInput, Privilege, PrivilegesNumber}, set_privileges};
use sqlx::{PgPool, query};
use uuid::{uuid, Uuid};

const ADIMAC_ID: Uuid = uuid!("ba34ff10-4b89-44cb-9b36-31eb57c41556");
const HUBERT_ID: Uuid = uuid!("263541a8-fa1e-4f13-9e5d-5b250a5a71e6");
const SOME_USER_ID: Uuid = uuid!("e287ccab-fb33-4314-8d81-bfa9d6e52928");
const CHADDERS_ID: Uuid = uuid!("b8c9a317-a456-458f-af88-01d99633f8e2");

async fn select_privileges(pg: &PgPool, group_id: Uuid) -> Result<HashMap<Role, i32>, sqlx::Error> {
    let query_res = query!(
        r#"
            SELECT role_type AS "role: Role", privileges
            FROM group_roles
            WHERE group_id = $1
        "#,
        group_id,
    ).fetch_all(pg).await?;

    let res = HashMap::from_iter(query_res.into_iter().map(|x| (x.role, x.privileges)));

    Ok(res)
}

#[sqlx::test(fixtures("roles/set_privileges"))]
async fn change_privileges(pg: PgPool) {
    let mut rd = tools::add_redis::<Vec<String>>(1, vec![]).await;

    set_privileges(&pg, &mut rd, ADIMAC_ID, &PrivilegeChangeInput::new(CHADDERS_ID, Role::Admin, Privilege::CanInvite(false))).await.unwrap();

    let privileges = select_privileges(&pg, CHADDERS_ID).await.unwrap();

    dbg!(&privileges);
    assert_eq!(privileges.get(&Role::Admin).copied(), Some(2))
}

#[sqlx::test(fixtures("roles/set_privileges"))]
async fn change_privileges_insufficient_role(pg: PgPool) {
    let mut rd = tools::add_redis::<Vec<String>>(2, vec![]).await;

    let res = set_privileges(&pg, &mut rd, HUBERT_ID, &PrivilegeChangeInput::new(CHADDERS_ID, Role::Admin, Privilege::CanInvite(false))).await;

    assert!(res.is_err());
}
