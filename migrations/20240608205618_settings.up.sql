create table if not exists settings(
  id                text not null primary key default 'DEFAULT_SETTINGS',
  encrypted_api_key text not null
);

insert into settings (encrypted_api_key) values ('<your api key here>')
