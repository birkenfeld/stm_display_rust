pub const BOOTUP: &[&[u8]] = &[
    b"Allocating PCI resources starting at cc000000 (gap: cc000000:24000000)",
    b"Booting paravirtualized kernel on bare hardware",
    b"NR_CPUS:4096 nr_cpumask_bits:8 nr_cpu_ids:8 nr_node_ids:1",
    b"PERCPU: Embedded 33 pages/cpu @ffff880028200000 s104088 r8192 d22888 u262144",
    b"pcpu-alloc: s104088 r8192 d22888 u262144 alloc=1*2097152",
    b"pcpu-alloc: [0] 0 1 2 3 4 5 6 7 ",
    b"Built 1 zonelists in Zone order, mobility grouping on.  Total pages: 2047268",
    b"Policy zone: Normal",
    b"Kernel command line: ro root=LABEL=/ rd_NO_LUKS rd_NO_LVM LANG=en_US.UTF-8 rd_NO_MD SYSFONT=latarcyrheb-sun16  KEYBOARDTYPE=pc KEYTABLE=us rd_NO_DM rhgb quiet nouveau.modeset=0 rdblacklist=nouveau crashkernel=129M@48M",
    b"PID hash table entries: 4096 (order: 3, 32768 bytes)",
    b"x86/fpu: xstate_offset[2]: 0240, xstate_sizes[2]: 0100",
    b"xsave: enabled eager FPU xstate_bv 0x7, cntxt size 0x340",
    b"dmar: Queued invalidation will be enabled to support x2apic and Intr-remapping.",
    b"Memory: 7873748k/9175040k available (5525k kernel code, 860056k absent, 441236k reserved, 6904k data, 1340k init)",
    b"Kernel/User page tables isolation: enabled",
    b"Hierarchical RCU implementation.",
    b"NR_IRQS:33024 nr_irqs:880",
    b"Extended CMOS year: 2000",
    b"Console: colour dummy device 80x25",
    b"console [tty0] enabled",
    b"allocated 33554432 bytes of page_cgroup",
    b"please try 'cgroup_disable=memory' option if you don't want memory cgroups",
    b"hpet clockevent registered",
    b"TSC: cpu family 6 model 45, tsc initial value = 2f3f3abebc",
    b"Fast TSC calibration using PIT",
    b"Detected 3591.310 MHz processor.",
    b"Calibrating delay loop (skipped), value calculated using timer frequency.. 7182.62 BogoMIPS (lpj=3591310)",
    b"pid_max: default: 32768 minimum: 301",
    b"Security Framework initialized",
    b"SELinux:  Initializing.",
    b"SELinux:  Starting in permissive mode",
    b"Dentry cache hash table entries: 1048576 (order: 11, 8388608 bytes)",
    b"Inode-cache hash table entries: 524288 (order: 10, 4194304 bytes)",
    b"Mount-cache hash table entries: 256",
    b"Initializing cgroup subsys ns",
    b"Initializing cgroup subsys cpuacct",
    b"Initializing cgroup subsys memory",
    b"Initializing cgroup subsys devices",
    b"Initializing cgroup subsys freezer",
    b"Initializing cgroup subsys net_cls",
    b"Initializing cgroup subsys blkio",
    b"Initializing cgroup subsys perf_event",
    b"Initializing cgroup subsys net_prio",
    b"CPU: Physical Processor ID: 0",
    b"CPU: Processor Core ID: 0",
    b"mce: CPU supports 16 MCE banks",
    b"CPU0: Thermal monitoring enabled (TM1)",
    b"using mwait in idle threads.",
    b"Speculative Store Bypass: Vulnerable",
    b"FEATURE SPEC_CTRL Not Present",
    b"FEATURE IBPB_SUPPORT Not Present",
    b"Spectre V2 : Mitigation: Full retpoline",
    b"ACPI: Core revision 20090903",
    b"ftrace: converting mcount calls to 0f 1f 44 00 00",
    b"ftrace: allocating 22056 entries in 87 pages",
    b"dmar: Host address width 46",
    b"dmar: DRHD base: 0x000000ef944000 flags: 0x1",
    b"dmar: IOMMU 0: reg_base_addr ef944000 ver 1:0 cap d2078c106f0462 ecap f020fe",
    b"dmar: RMRR base: 0x000000cb6b8000 end: 0x000000cb6e3fff",
    b"dmar: ATSR flags: 0x0",
    b"IOAPIC id 0 under DRHD base 0xef944000",
    b"IOAPIC id 2 under DRHD base 0xef944000",
    b"HPET id 0 under DRHD base 0xef944000",
    b"Enabled IRQ remapping in x2apic mode",
    b"Enabling x2apic",
    b"Enabled x2apic",
    b"APIC routing finalized to cluster x2apic.",
    b"  alloc irq_desc for 48 on node 0",
    b"  alloc kstat_irqs on node 0",
    b"alloc irq_2_iommu on node 0",
    b"Initializing cgroup subsys ns",
    b"Initializing cgroup subsys cpuacct",
    b"Initializing cgroup subsys memory",
    b"Initializing cgroup subsys devices",
    b"Initializing cgroup subsys freezer",
    b"Initializing cgroup subsys net_cls",
    b"Initializing cgroup subsys blkio",
    b"Initializing cgroup subsys perf_event",
    b"Initializing cgroup subsys net_prio",
    b"CPU: Physical Processor ID: 0",
    b"CPU: Processor Core ID: 0",
    b"mce: CPU supports 16 MCE banks",
    b"CPU0: Thermal monitoring enabled (TM1)",
    b"using mwait in idle threads.",
    b"Speculative Store Bypass: Vulnerable",
    b"FEATURE SPEC_CTRL Not Present",
    b"FEATURE IBPB_SUPPORT Not Present",
    b"Spectre V2 : Mitigation: Full retpoline",
    b"ACPI: Core revision 20090903",
    b"ftrace: converting mcount calls to 0f 1f 44 00 00",
    b"ftrace: allocating 22056 entries in 87 pages",
    b"dmar: Host address width 46",
    b"dmar: DRHD base: 0x000000ef944000 flags: 0x1",
    b"dmar: IOMMU 0: reg_base_addr ef944000 ver 1:0 cap d2078c106f0462 ecap f020fe",
    b"dmar: RMRR base: 0x000000cb6b8000 end: 0x000000cb6e3fff",
    b"dmar: ATSR flags: 0x0",
    b"IOAPIC id 0 under DRHD base 0xef944000",
    b"IOAPIC id 2 under DRHD base 0xef944000",
    b"HPET id 0 under DRHD base 0xef944000",
    b"Enabled IRQ remapping in x2apic mode",
    b"Enabling x2apic",
    b"Enabled x2apic",
    b"APIC routing finalized to cluster x2apic.",
    b"  alloc irq_desc for 48 on node 0",
    b"  alloc kstat_irqs on node 0",
    b"alloc irq_2_iommu on node 0",
    b"..TIMER: vector=0x30 apic1=0 pin1=2 apic2=-1 pin2=-1",
    b"CPU0: Intel(R) Xeon(R) CPU E5-1620 0 @ 3.60GHz stepping 07",
    b"Performance Events: PEBS fmt1+, 16-deep LBR, SandyBridge events, full-width counters, Intel PMU driver.",
    b"... version:                3",
    b"... bit width:              48",
    b"... generic registers:      4",
    b"... value mask:             0000ffffffffffff",
    b"... max period:             0000ffffffffffff",
    b"... fixed-purpose events:   3",
    b"... event mask:             000000070000000f",
    b"NMI watchdog enabled, takes one hw-pmu counter.",
    b"Booting Node   0, Processors  #1 #2 #3 #4 #5 #6 #7 Ok.",
    b"Brought up 8 CPUs",
    b"Total of 8 processors activated (57460.96 BogoMIPS).",
    b"sizeof(vma)=200 bytes",
    b"sizeof(page)=56 bytes",
    b"sizeof(inode)=592 bytes",
    b"sizeof(dentry)=192 bytes",
    b"sizeof(ext3inode)=800 bytes",
    b"sizeof(buffer_head)=104 bytes",
    b"sizeof(skbuff)=232 bytes",
    b"sizeof(task_struct)=2672 bytes",
    b"devtmpfs: initialized",
    b"PM: Registering ACPI NVS region at cb7fd000 (1884160 bytes)",
    b"PM: Registering ACPI NVS region at cbaeb000 (548864 bytes)",
    b"regulator: core version 0.5",
    b"NET: Registered protocol family 16",
    b"ACPI FADT declares the system doesn't support PCIe ASPM, so disable it",
    b"ACPI: bus type pci registered",
    b"PCI: MCFG configuration 0: base f0000000 segment 0 buses 0 - 127",
    b"PCI: MCFG area at f0000000 reserved in E820",
    b"PCI: Using MMCONFIG at f0000000 - f7ffffff",
    b"PCI: Using configuration type 1 for base access",
    b"PMU erratum BJ122, BV98, HSD29 worked around, HT is on",
    b"bio: create slab <bio-0> at 0",
    b"ACPI: EC: Look up EC in DSDT",
    b"ACPI: Executed 1 blocks of module-level executable AML code",
    b"ACPI: Interpreter enabled",
    b"ACPI: (supports S0 S3 S4 S5)",
    b"ACPI: Using IOAPIC for interrupt routing",
    b"ACPI: No dock devices found.",
    b"PCI: Using host bridge windows from ACPI; if necessary, use \"pci=nocrs\" and report a bug",
    b"ACPI: PCI Root Bridge [PCI0] (domain 0000 [bus 00-3f])",
];

pub const DISPLAY: &[(&[u8], &[u8], bool)] = &[
    (b"15.421", b"40.008", true),
    (b"15.527", b"40.056", true),
    (b"15.539", b"40.041", true),
    (b"15.442", b"40.027", true),
    (b"15.571", b"40.012", true),
    (b"15.472", b"40.004", true),
    (b"15.508", b"39.971", false),
    (b"15.517", b"39.954", false),
    (b"15.493", b"39.913", false),
    (b"15.403", b"39.907", false),
    (b"15.504", b"39.938", false),
    (b"15.499", b"39.979", false),
];
