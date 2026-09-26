
def notice_filenames(name):
    return (name + ".NOTICE", name + ".NOTICE.sha256")

def notice_root_error(root):
    if type(root) != "string" or root == "":
        return ("notice_bundle: invalid root '" + str(root) +
                "': want a non-empty distributed-tier root label")
    return ""

def notice_manifest_error(manifest):
    if manifest == None:
        return "notice_bundle: invalid manifest 'None': want the audited inventory manifest label"
    return ""

def notice_bundle(name, root, inventory, texts):
    root_err = notice_root_error(root)
    if root_err != "":
        fail(root_err + " (in " + native.package_name() + ":" + name + ")")
    manifest_err = notice_manifest_error(inventory)
    if manifest_err != "":
        fail(manifest_err + " (in " + native.package_name() + ":" + name + ")")
    if len(texts) == 0:
        fail("notice_bundle " + native.package_name() + ":" + name +
             ": need at least one license-words file")
    (notice, checksum) = notice_filenames(name)

    native.genrule(
        name = name + "_notice",
        srcs = [inventory] + texts,
        outs = [notice],
        tools = ["//deploy/release:notice_gen"],
        cmd = "$(location //deploy/release:notice_gen) $(location " + inventory + ") $(OUTS) \"" + root + "\" " +
              " ".join(["$(location " + text + ")" for text in texts]),
    )

    native.genrule(
        name = name + "_checksum",
        srcs = [":" + name + "_notice"],
        outs = [checksum],
        tools = ["//deploy/rules:hasher"],
        cmd = "$(location //deploy/rules:hasher) $(location :" + name + "_notice) $(OUTS)",
    )

    native.filegroup(
        name = name,
        srcs = [":" + name + "_notice", ":" + name + "_checksum"],
    )
