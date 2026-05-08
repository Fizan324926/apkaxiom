#!/usr/bin/env python3
"""
scan_app_vulns.py — Real APK vulnerability scanner.

Scans Android APKs for exploitable app-level vulnerabilities:
  1. Exported components without permission protection
  2. ContentProviders with potential path traversal (openFile)
  3. WebView with addJavascriptInterface on external URLs
  4. Intent redirection (getParcelableExtra → startActivity)
  5. Insecure deep links (custom schemes without validation)
  6. Backup enabled without encryption (allowBackup=true + sensitive perms)

Two-pass architecture:
  Pass 1 (fast): Manifest-only analysis for all 1000 APKs
  Pass 2 (deep): DEX bytecode analysis for apps flagged in pass 1

Outputs JSON report to test/cve/app_vuln_results.json.
"""

import json
import os
import sys
import time
import traceback
from collections import defaultdict
from pathlib import Path
from concurrent.futures import ProcessPoolExecutor, as_completed
from typing import Any

# Suppress androguard's loguru noise before any import
from loguru import logger
logger.remove()
logger.add(sys.stderr, level="CRITICAL")


CORPUS_DIR = Path(__file__).resolve().parent.parent.parent / "corpus" / "bench-10k" / "real-fdroid"
OUTPUT_FILE = Path(__file__).resolve().parent / "app_vuln_results.json"

NS = "{http://schemas.android.com/apk/res/android}"

# Permissions that indicate the app handles sensitive data
SENSITIVE_PERMISSIONS = {
    "android.permission.READ_CONTACTS",
    "android.permission.WRITE_CONTACTS",
    "android.permission.READ_CALENDAR",
    "android.permission.WRITE_CALENDAR",
    "android.permission.READ_CALL_LOG",
    "android.permission.WRITE_CALL_LOG",
    "android.permission.READ_SMS",
    "android.permission.SEND_SMS",
    "android.permission.RECEIVE_SMS",
    "android.permission.CAMERA",
    "android.permission.RECORD_AUDIO",
    "android.permission.ACCESS_FINE_LOCATION",
    "android.permission.ACCESS_COARSE_LOCATION",
    "android.permission.READ_EXTERNAL_STORAGE",
    "android.permission.WRITE_EXTERNAL_STORAGE",
    "android.permission.READ_PHONE_STATE",
    "android.permission.MANAGE_ACCOUNTS",
    "android.permission.GET_ACCOUNTS",
    "android.permission.AUTHENTICATE_ACCOUNTS",
    "android.permission.USE_CREDENTIALS",
}

# Component names from common libraries — not app-specific, lower severity
LIBRARY_COMPONENT_PREFIXES = (
    "com.google.",
    "androidx.",
    "android.support.",
    "com.facebook.",
    "com.crashlytics.",
    "io.fabric.",
    "com.android.installreferrer.",
    "com.microsoft.",
    "com.huawei.",
)


def classify_severity(vuln_type: str, component_name: str, details: dict) -> str:
    """Assign HIGH / MEDIUM / LOW based on exploitability."""
    is_library = any(component_name.startswith(p) for p in LIBRARY_COMPONENT_PREFIXES)

    if vuln_type == "INTENT_REDIRECT":
        return "HIGH"  # Always high — allows launching arbitrary components
    if vuln_type == "PROVIDER_PATH_TRAVERSAL":
        return "HIGH"  # File read/write
    if vuln_type == "WEBVIEW_JS_INTERFACE":
        if details.get("loads_external_urls"):
            return "HIGH"
        return "MEDIUM"
    if vuln_type == "EXPORTED_NO_PERM":
        comp_type = details.get("component_type", "")
        if comp_type == "provider":
            return "HIGH"  # Exported provider without perm = data leak
        if comp_type == "service" and not is_library:
            return "HIGH"
        if comp_type == "receiver" and not is_library:
            return "MEDIUM"
        if is_library:
            return "LOW"
        return "MEDIUM"
    if vuln_type == "INSECURE_DEEPLINK":
        if details.get("custom_scheme") and not details.get("has_host_validation"):
            return "MEDIUM"
        return "LOW"
    if vuln_type == "BACKUP_SENSITIVE":
        if details.get("sensitive_perm_count", 0) >= 3:
            return "MEDIUM"
        return "LOW"
    return "LOW"


def scan_manifest(apk_path: str) -> dict:
    """
    Pass 1: Manifest-only scan. Fast — no DEX parsing.
    Returns dict of findings for this APK.
    """
    from androguard.core.apk import APK

    result = {
        "apk": os.path.basename(apk_path),
        "package": "",
        "version_name": "",
        "target_sdk": "",
        "findings": [],
        "error": None,
    }

    try:
        a = APK(apk_path)
    except Exception as e:
        result["error"] = f"APK parse failed: {str(e)[:200]}"
        return result

    try:
        result["package"] = a.get_package() or ""
        result["version_name"] = a.get_androidversion_name() or ""
        result["target_sdk"] = a.get_effective_target_sdk_version() or ""
    except:
        pass

    try:
        xml = a.get_android_manifest_xml()
    except Exception as e:
        result["error"] = f"Manifest parse failed: {str(e)[:200]}"
        return result

    permissions = set()
    try:
        permissions = set(a.get_permissions())
    except:
        pass

    # --- Check 1: Exported components without permission ---
    for tag in ["activity", "service", "receiver", "provider"]:
        for elem in xml.iter(tag):
            name = elem.get(NS + "name", "")
            exported_attr = elem.get(NS + "exported", "")
            permission = elem.get(NS + "permission", "")
            read_perm = elem.get(NS + "readPermission", "")
            write_perm = elem.get(NS + "writePermission", "")

            intent_filters = list(elem.iter("intent-filter"))
            has_filter = len(intent_filters) > 0

            # Determine if component is exported
            if exported_attr == "true":
                is_exported = True
            elif exported_attr == "false":
                is_exported = False
            else:
                # Default: exported if has intent-filter (pre-Android 12 behavior)
                is_exported = has_filter

            if not is_exported:
                continue

            # Check for any permission protection
            has_perm = bool(permission or read_perm or write_perm)

            # For providers, also check grantUriPermissions
            grant_uri = elem.get(NS + "grantUriPermissions", "")

            if not has_perm:
                finding = {
                    "type": "EXPORTED_NO_PERM",
                    "component": name,
                    "component_type": tag,
                    "has_intent_filter": has_filter,
                    "exported_explicit": exported_attr == "true",
                }
                if tag == "provider":
                    # Check authorities
                    authorities = elem.get(NS + "authorities", "")
                    finding["authorities"] = authorities
                    finding["grant_uri_permissions"] = grant_uri == "true"
                    # Check for path-permission children
                    path_perms = list(elem.iter("path-permission"))
                    finding["has_path_permissions"] = len(path_perms) > 0

                finding["severity"] = classify_severity("EXPORTED_NO_PERM", name, finding)
                result["findings"].append(finding)

    # --- Check 5: Insecure deep links ---
    for elem in xml.iter("activity"):
        name = elem.get(NS + "name", "")
        for intent_filter in elem.iter("intent-filter"):
            schemes = []
            hosts = []
            has_view_action = False
            has_browsable = False

            for action in intent_filter.iter("action"):
                if action.get(NS + "name") == "android.intent.action.VIEW":
                    has_view_action = True
            for category in intent_filter.iter("category"):
                if category.get(NS + "name") == "android.intent.category.BROWSABLE":
                    has_browsable = True
            for data in intent_filter.iter("data"):
                scheme = data.get(NS + "scheme", "")
                host = data.get(NS + "host", "")
                if scheme:
                    schemes.append(scheme)
                if host:
                    hosts.append(host)

            if has_view_action and has_browsable and schemes:
                # Filter out standard http/https — we want custom schemes
                custom_schemes = [s for s in schemes if s not in ("http", "https", "geo", "tel", "mailto", "sms")]
                if custom_schemes:
                    finding = {
                        "type": "INSECURE_DEEPLINK",
                        "component": name,
                        "schemes": custom_schemes,
                        "hosts": hosts,
                        "custom_scheme": True,
                        "has_host_validation": bool(hosts),
                        "severity": "MEDIUM",
                    }
                    finding["severity"] = classify_severity("INSECURE_DEEPLINK", name, finding)
                    result["findings"].append(finding)

                # Also flag http/https with no host constraint — open redirect risk
                web_schemes = [s for s in schemes if s in ("http", "https")]
                if web_schemes and not hosts:
                    finding = {
                        "type": "INSECURE_DEEPLINK",
                        "component": name,
                        "schemes": web_schemes,
                        "hosts": [],
                        "custom_scheme": False,
                        "has_host_validation": False,
                        "note": "Handles all HTTP/HTTPS URLs without host restriction",
                        "severity": "MEDIUM",
                    }
                    result["findings"].append(finding)

    # --- Check 6: Backup enabled with sensitive data ---
    app_elem = xml.find("application")
    if app_elem is not None:
        allow_backup = app_elem.get(NS + "allowBackup", "")
        full_backup_content = app_elem.get(NS + "fullBackupContent", "")

        # allowBackup defaults to true if not specified
        backup_enabled = allow_backup != "false"

        if backup_enabled:
            sensitive_perms = permissions & SENSITIVE_PERMISSIONS
            if sensitive_perms:
                finding = {
                    "type": "BACKUP_SENSITIVE",
                    "allow_backup_explicit": allow_backup == "true",
                    "has_full_backup_rules": bool(full_backup_content),
                    "sensitive_permissions": sorted(sensitive_perms),
                    "sensitive_perm_count": len(sensitive_perms),
                }
                finding["severity"] = classify_severity("BACKUP_SENSITIVE", "", finding)
                result["findings"].append(finding)

    return result


def scan_dex_deep(apk_path: str, manifest_findings: list) -> list:
    """
    Pass 2: DEX bytecode analysis for high-value targets.
    Looks for:
      - addJavascriptInterface calls (WebView JS bridge)
      - getParcelableExtra → startActivity chains (intent redirect)
      - openFile overrides in ContentProviders
    """
    from androguard.misc import AnalyzeAPK

    extra_findings = []

    try:
        a, d_list, dx = AnalyzeAPK(apk_path)
    except Exception:
        return extra_findings

    if dx is None:
        return extra_findings

    package = a.get_package() or ""

    # --- Check 3: WebView addJavascriptInterface ---
    try:
        for meth_analysis in dx.find_methods(
            classname="Landroid/webkit/WebView;",
            methodname="addJavascriptInterface"
        ):
            for ref_class, ref_meth, offset in meth_analysis.get_xref_from():
                caller_class = ref_meth.get_class_name()
                caller_method = ref_meth.get_name()

                # Check if this class also calls loadUrl with non-local URLs
                loads_external = False
                try:
                    for load_meth in dx.find_methods(
                        classname="Landroid/webkit/WebView;",
                        methodname="loadUrl"
                    ):
                        for ref_c2, ref_m2, _ in load_meth.get_xref_from():
                            if ref_m2.get_class_name() == caller_class:
                                loads_external = True
                                break
                except:
                    pass

                finding = {
                    "type": "WEBVIEW_JS_INTERFACE",
                    "component": caller_class.replace("/", ".").strip("L;"),
                    "caller_method": caller_method,
                    "loads_external_urls": loads_external,
                }
                finding["severity"] = classify_severity("WEBVIEW_JS_INTERFACE", finding["component"], finding)
                extra_findings.append(finding)
    except Exception:
        pass

    # --- Check 4: Intent redirection ---
    # Pattern: getIntent() or getParcelableExtra() followed by startActivity/startService
    try:
        # Find classes that call both getParcelableExtra and startActivity
        parcelable_callers = set()
        start_callers = set()

        for meth_analysis in dx.find_methods(
            classname="Landroid/content/Intent;",
            methodname="getParcelableExtra"
        ):
            for _, ref_meth, _ in meth_analysis.get_xref_from():
                parcelable_callers.add(ref_meth.get_class_name())

        # Also check getExtras
        for meth_analysis in dx.find_methods(
            classname="Landroid/content/Intent;",
            methodname="getExtras"
        ):
            for _, ref_meth, _ in meth_analysis.get_xref_from():
                parcelable_callers.add(ref_meth.get_class_name())

        for method_name in ["startActivity", "startActivities", "startService", "sendBroadcast"]:
            for meth_analysis in dx.find_methods(
                classname="Landroid/content/Context;",
                methodname=method_name
            ):
                for _, ref_meth, _ in meth_analysis.get_xref_from():
                    start_callers.add((ref_meth.get_class_name(), method_name))
            # Also check Activity's own startActivity
            for meth_analysis in dx.find_methods(
                classname="Landroid/app/Activity;",
                methodname=method_name
            ):
                for _, ref_meth, _ in meth_analysis.get_xref_from():
                    start_callers.add((ref_meth.get_class_name(), method_name))

        # Intersection: classes that both read an Intent extra AND start a component
        for caller_class, dispatch_method in start_callers:
            if caller_class in parcelable_callers:
                class_name = caller_class.replace("/", ".").strip("L;")
                # Only flag app classes, not library classes
                if any(class_name.startswith(p.rstrip(".")) for p in LIBRARY_COMPONENT_PREFIXES):
                    continue
                finding = {
                    "type": "INTENT_REDIRECT",
                    "component": class_name,
                    "dispatch_method": dispatch_method,
                    "note": "Class reads parcelable/extras from incoming Intent and dispatches to another component",
                }
                finding["severity"] = classify_severity("INTENT_REDIRECT", class_name, finding)
                extra_findings.append(finding)
    except Exception:
        pass

    # --- Check 2: ContentProvider openFile path traversal ---
    # Find classes that extend ContentProvider and override openFile
    try:
        # Get all exported providers from manifest findings
        exported_providers = set()
        for f in manifest_findings:
            if f.get("type") == "EXPORTED_NO_PERM" and f.get("component_type") == "provider":
                provider_name = f.get("component", "")
                # Convert Java class name to dalvik format
                dalvik_name = "L" + provider_name.replace(".", "/") + ";"
                exported_providers.add(dalvik_name)

        for meth_analysis in dx.find_methods(methodname="openFile"):
            meth = meth_analysis.get_method()
            class_name = meth.get_class_name()

            # Check if this is an app ContentProvider
            if class_name in exported_providers:
                # Check if the method body contains any path validation
                has_validation = False
                try:
                    encoded_meth = meth
                    if hasattr(encoded_meth, 'get_instructions'):
                        for inst in encoded_meth.get_instructions():
                            output = inst.get_output()
                            if output and (".." in output or "canonical" in output.lower() or "normalize" in output.lower()):
                                has_validation = True
                                break
                except:
                    pass

                finding = {
                    "type": "PROVIDER_PATH_TRAVERSAL",
                    "component": class_name.replace("/", ".").strip("L;"),
                    "has_path_validation": has_validation,
                    "note": "Exported ContentProvider overrides openFile() without permission protection",
                }
                if not has_validation:
                    finding["severity"] = "HIGH"
                    finding["exploit"] = "Send content:// URI with ../../ path segments to read arbitrary files"
                else:
                    finding["severity"] = "MEDIUM"
                extra_findings.append(finding)
    except Exception:
        pass

    return extra_findings


def process_single_apk(apk_path: str) -> dict:
    """Process a single APK: manifest scan + conditional DEX scan."""
    # Suppress loguru in worker process
    from loguru import logger as worker_logger
    worker_logger.remove()

    result = scan_manifest(apk_path)

    # Decide if DEX scan is warranted
    needs_dex = False

    # DEX scan if we have exported components (check for intent redirect, JS interface)
    has_exported_activity = any(
        f["type"] == "EXPORTED_NO_PERM" and f["component_type"] in ("activity", "service")
        for f in result["findings"]
    )
    has_exported_provider = any(
        f["type"] == "EXPORTED_NO_PERM" and f["component_type"] == "provider"
        for f in result["findings"]
    )
    has_deeplink = any(f["type"] == "INSECURE_DEEPLINK" for f in result["findings"])

    # Always do DEX scan for apps with exported providers, deep links, or many exported components
    exported_count = sum(1 for f in result["findings"] if f["type"] == "EXPORTED_NO_PERM")
    if has_exported_provider or has_deeplink or exported_count >= 2:
        needs_dex = True

    if needs_dex:
        try:
            dex_findings = scan_dex_deep(apk_path, result["findings"])
            result["findings"].extend(dex_findings)
        except Exception as e:
            if result.get("error"):
                result["error"] += f"; DEX scan failed: {str(e)[:100]}"
            else:
                result["error"] = f"DEX scan failed: {str(e)[:100]}"

    return result


def deduplicate_findings(findings: list) -> list:
    """Remove duplicate findings (same type + component)."""
    seen = set()
    deduped = []
    for f in findings:
        key = (f.get("type"), f.get("component", ""), f.get("component_type", ""))
        if key not in seen:
            seen.add(key)
            deduped.append(f)
    return deduped


def main():
    apk_dir = CORPUS_DIR
    if not apk_dir.exists():
        print(f"Corpus directory not found: {apk_dir}", file=sys.stderr)
        sys.exit(1)

    apk_files = sorted(apk_dir.glob("*.apk"))
    total = len(apk_files)
    print(f"Scanning {total} APKs in {apk_dir}")

    all_results = []
    errors = 0
    t0 = time.time()

    # Use multiprocessing for speed, but limit workers to avoid OOM
    max_workers = min(8, os.cpu_count() or 4)
    completed = 0

    with ProcessPoolExecutor(max_workers=max_workers) as pool:
        futures = {pool.submit(process_single_apk, str(p)): p for p in apk_files}

        for future in as_completed(futures):
            completed += 1
            try:
                result = future.result(timeout=120)
                result["findings"] = deduplicate_findings(result["findings"])
                all_results.append(result)
                if result["error"]:
                    errors += 1
                finding_count = len(result["findings"])
                if finding_count > 0 and completed % 50 == 0:
                    elapsed = time.time() - t0
                    print(f"  [{completed}/{total}] {elapsed:.1f}s elapsed, "
                          f"last: {result['apk']} ({finding_count} findings)")
            except Exception as e:
                errors += 1
                apk_name = os.path.basename(str(futures[future]))
                all_results.append({
                    "apk": apk_name,
                    "package": "",
                    "findings": [],
                    "error": f"Worker failed: {str(e)[:200]}"
                })
                if completed % 100 == 0:
                    print(f"  [{completed}/{total}] error on {apk_name}")

    elapsed = time.time() - t0
    print(f"\nScan complete: {total} APKs in {elapsed:.1f}s ({errors} errors)")

    # --- Aggregate statistics ---
    all_findings = []
    for r in all_results:
        for f in r["findings"]:
            f["apk"] = r["apk"]
            f["package"] = r.get("package", "")
            all_findings.append(f)

    # Count by type and severity
    by_type = defaultdict(int)
    by_severity = defaultdict(int)
    high_findings = []

    for f in all_findings:
        by_type[f["type"]] += 1
        sev = f.get("severity", "LOW")
        by_severity[sev] += 1
        if sev == "HIGH":
            high_findings.append(f)

    print(f"\nTotal findings: {len(all_findings)}")
    print(f"By type:")
    for t, c in sorted(by_type.items()):
        print(f"  {t}: {c}")
    print(f"By severity:")
    for s, c in sorted(by_severity.items()):
        print(f"  {s}: {c}")
    print(f"\nHIGH severity findings: {len(high_findings)}")

    for f in sorted(high_findings, key=lambda x: x["type"]):
        print(f"  [{f['type']}] {f['package']} / {f.get('component', 'N/A')}")

    # --- Build output report ---
    report = {
        "scan_metadata": {
            "corpus": str(apk_dir),
            "total_apks": total,
            "scan_time_seconds": round(elapsed, 1),
            "errors": errors,
            "scanner": "scan_app_vulns.py",
        },
        "summary": {
            "total_findings": len(all_findings),
            "by_type": dict(by_type),
            "by_severity": dict(by_severity),
            "high_count": len(high_findings),
        },
        "high_severity_findings": sorted(high_findings, key=lambda x: (x["type"], x["package"])),
        "all_findings_by_apk": {},
    }

    for r in sorted(all_results, key=lambda x: x["apk"]):
        if r["findings"]:
            report["all_findings_by_apk"][r["apk"]] = {
                "package": r.get("package", ""),
                "version": r.get("version_name", ""),
                "target_sdk": r.get("target_sdk", ""),
                "findings": r["findings"],
            }

    OUTPUT_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_FILE, "w") as fp:
        json.dump(report, fp, indent=2, default=str)

    print(f"\nReport written to {OUTPUT_FILE}")
    return report


if __name__ == "__main__":
    report = main()
