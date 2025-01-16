import daft
import sys

sql_query = sys.argv[1]
daft.context.set_runner_ray()
daft.sql(sql_query).show()
