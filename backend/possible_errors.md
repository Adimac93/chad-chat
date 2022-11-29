# Files

## utils/auth/mod.rs

`try_register_user()`

login or password is missing
password is too weak
a user with this login already exists

- [ ] password hashing fails
- [ ] query user_id by login fails
- [ ] inserting user into db fails

`login_user()`

login or password is missing
login does not match any result in db
password does not match with username

- [ ] query user_id by login fails

`authorize_user()`
- [ ] token encryption fails

## utils/chat/mod.rs

`get_user_login_by_id()`

query login by user_id does not find any row

- [ ] query login by user_id fails

`fetch_chat_messages()`

- [ ] query messages by group fails

`create_message()`

- [ ] inserting message into db fails

## routes/chat.rs

`chat_socket()`

UUID is invalid
group with this uuid does not exist
user is not a group member

- [ ] thread can fail to access the mutex lock

## utils/groups/mod.rs

`try_add_user_to_group()`

user is already in this group

- [ ] query/transaction fails

`create_group()`

group name is empty
user is already in this group (may never happen)

- [ ] query/transaction fails

`check_if_group_member()`

- [ ] query user by its id and group id fails

`query_user_groups()`

- [ ] query groups by user id fails

`check_if_group_exists()`

- [ ] query group by id fails

`get_group_info()`

- [ ] query group data fails

## routes/groups.rs

`post_create_group_invitation_link()`

- [ ] user is not in this group

`get_join_group_by_link()`

- [ ] wrong invitation link

`get_invitation_info()`

- [ ] wrong invitation link

## configuration.rs

`get_config()`

config build fails (probably something wrong with the file)
config deserialization fails

## database.rs

`get_database_pool()`

connection fails - panics

## main.rs

`main()`

- [ ] fails to run server

## models.rs

`claims - [ ] from_request()`

- [ ] invalid or expired jwt token

`try_register_user()`

registers the user with specified login credentials (adds the user to the database)

Should fail when:
- [x] at least one of the credentials is missing (or empty after trimming)
- [x] username is taken
- [x] password is weak
- [ ] suggestion: username is not between a specified range of length

`login_user()`

verifies login credentials
Should fail when:
- [x] at least one of the credentials is missing (or empty after trimming)
- [x] it can't select a corresponding username
- [x] password doesn't match the username

`authorize_user()`

creates a jwt token and sends it to the client (in a cookie jar)
Should fail when:
- [x] credential verification fails - login_user()

`get_user_login_by_id()`

queries for a user login
Should fail when:
- [x] it can't find the corresponding username

`create_message()`

saves a message in the database
Should fail when:
- [ ] its content is empty (when trimmed)
- [x] user is not authorized

`try_add_user_to_group()`

saves a record indicating the presence of a particular user in the group in the database
Should fail when:
- [x] user does not exist
- [x] group does not exist
- [x] user is already in group

`create_group()`

adds a new group to the database
Should fail when:
- [x] user is not authorized
- [x] group name is empty (when trimmed)

`check_if_group_member()`

checks if a particular user is a particular group member
This should never fail

`query_user_groups()`

searches every group, which user is a member of
Should fail when:
- [x] user is not authorized

`check_if_group_exists()`

checks whether a specified group exists
This should never fail

`get_group_info()`

searches the group information:
Should fail when:
- [x] group does not exist
- [x] user is not authorized

`post_create_group_invitation_link()`

creates an invitation link that can be sent back to the client
Should fail when:
- [ ] group does not exist
- [x] user is not authorized

`get_join_group_by_link()`

tries to join a specified group with a link
Should fail when:
- [ ] group does not exist
- [x] user is not authorized
- [ ] invitation link does not match the correct one

`get_invitation_info()`

gets information about a group that could be shown to a user with the invitation link
Should fail when:
- [ ] group does not exist
- [x] user is not authorized
- [ ] invitation link does not match the correct one