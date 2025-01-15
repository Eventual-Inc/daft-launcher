import daft
import sys

sql_query = sys.argv[1]
daft.sql(sql_query).show()

