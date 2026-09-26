def is_canonical(text):
    return text.startswith("@@")  # buildifier: disable=canonical-repository

def strip_canonical(text):
    if is_canonical(text):
        return text[2:]
    return text
