"""Concrete action-subjects use case."""

def admitted_actions():
    return ["StarlarkAction", "FileWrite"]

def admitted_action_outputs():
    return ["out.txt", "report.txt"]

def resolve_action(mnemonic, mapping):
    if mnemonic in mapping:
        return mapping[mnemonic]
    return "no action for '" + mnemonic + "': want one of " + ", ".join(sorted(mapping.keys()))

def action_report(mnemonic, outputs):
    return "Action " + mnemonic + " produced " + str(len(outputs)) + " outputs: " + ", ".join(outputs) + "!"

def action_subject_fields(mnemonic, outputs):
    return {
        "mnemonic": mnemonic,
        "outputs": ", ".join(outputs),
        "report": action_report(mnemonic, outputs),
    }

def action_fingerprint_like(mnemonic):
    return json.encode({
        "mnemonic": mnemonic,
        "outputs": ["out.txt"],
    })

def is_supported_action(mnemonic):
    return mnemonic in admitted_actions()
