import json
import warnings

# warning raised by pkg_resources used in a lot of google packages
warnings.filterwarnings("ignore", message=r".*declare_namespace\(\'.*google.*", category=DeprecationWarning)
# base warning raised when warning above are raised
warnings.filterwarnings("ignore", message=r".*pkg_resources is deprecated.*", category=DeprecationWarning)

# must import taggers to register them
# we import the rust extension here and wrap it in a python module
from . import dolma as _dolma  # type: ignore   # noqa: E402
from . import taggers  # noqa: E402


def deduper(config: dict):
    return _dolma.deduper_entrypoint(json.dumps(config))


def mixer(config: dict):
    return _dolma.mixer_entrypoint(json.dumps(config))
