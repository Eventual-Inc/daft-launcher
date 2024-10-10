## Sql

Daft now supports a SQL API.
This means that you can run raw SQL queries against your data using daft.
The SQL dialect is the `postgres` standard.

### Example

```bash
# run a sql query using the default .daft.toml configuration file
daft sql -- "\"SELECT * FROM my_table\""

# or, if you want, establish the port-forward using a custom configuration file
daft sql -c my-custom-config.toml -- "\"SELECT * FROM my_table\""
```
