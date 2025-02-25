from argparse import ArgumentParser
from pathlib import Path
from typing import List, Optional

from yaml import safe_load

from .deduper import DeduperCli
from .mixer import MixerCli
from .tagger import TaggerCli

AVAILABLE_COMMANDS = {
    "dedupe": DeduperCli,
    "mix": MixerCli,
    "tag": TaggerCli,
    # "visualize": None,
    # "browse": None,
    # "stats": None,
    # "ft-train": None,
}


def main(argv: Optional[List[str]] = None):
    parser = ArgumentParser(
        prog="dolma",
        usage="dolma [command] [options]",
        description="Command line interface for the DOLMa dataset processing toolkit",
    )
    parser.add_argument(
        "-c",
        "--config",
        help="Path to configuration optional file",
        type=Path,
        default=None,
    )
    subparsers = parser.add_subparsers(dest="command")
    subparsers.required = True
    subparsers.choices = AVAILABLE_COMMANDS.keys()  # type: ignore

    for command, cli in AVAILABLE_COMMANDS.items():
        if cli is not None:
            cli.make_parser(subparsers.add_parser(command))

    args = parser.parse_args(argv)

    # try parsing the config file
    config: Optional[dict] = None
    if config_path := args.__dict__.pop("config"):
        assert config_path.exists(), f"Config file {config_path} does not exist"
        with open(config_path) as f:
            config = dict(safe_load(f))

    AVAILABLE_COMMANDS[args.__dict__.pop("command")].run_from_args(args=args, config=config)
