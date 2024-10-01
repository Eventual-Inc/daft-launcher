import argparse
import daft


def main(sql: str):
    daft.context.set_runner_ray()
    df = daft.sql(sql).collect()
    df.show()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("sql")
    args = parser.parse_args()
    main(args.sql)
