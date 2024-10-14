from daft_launcher import data_definitions


def replace_with_pre_configured_template(
    aws_configuration: data_definitions.CustomConfiguration,
) -> data_definitions.CustomConfiguration:
    new_setup = data_definitions.Setup(**aws_configuration.setup.dict(exclude={}))
    return data_definitions.CustomConfiguration(
        daft_launcher_version=aws_configuration.daft_launcher_version,
        setup=new_setup,
        run=data_definitions.Run(
            pre_setup_commands=[],
            setup_commands=[],
        ),
    )
