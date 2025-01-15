import daft

daft.context.set_runner_ray()

df = daft.from_pydict({ "nums": [1,2,3] })
df.agg(daft.col("nums").mean()).show()
