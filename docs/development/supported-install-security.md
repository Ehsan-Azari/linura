# Supported-install security checklist

The `arch-hyprland-v1` development profile does not qualify as supported until tests demonstrate all of the following on a clean install:

- encrypted root/storage according to the profile;
- inbound firewall deny-by-default;
- SSH disabled until explicitly enabled through an approved intent/action;
- untrusted package sources disabled until explicitly enabled;
- baseline snapshot/factory-reset anchor when Btrfs/Snapper is selected;
- native shell/package-manager recovery path without a model provider;
- update guard does not prevent deliberate break-glass repair;
- first boot can complete using deterministic defaults or imported profile when offline.
