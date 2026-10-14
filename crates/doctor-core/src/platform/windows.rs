//! Windows FFI boundary. Owned buffers are aligned; API-owned buffers/handles are
//! released by RAII. No shell commands, registry writes or elevation.
use crate::{models::*, Error, Result};
use std::{
    ffi::c_void,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, GlobalFree, HANDLE, INVALID_HANDLE_VALUE},
    NetworkManagement::{IpHelper::*, WiFi::*},
    Networking::{WinHttp::*, WinSock::*},
};

struct IcmpHandle(HANDLE);
impl Drop for IcmpHandle {
    fn drop(&mut self) {
        unsafe {
            IcmpCloseHandle(self.0);
        }
    }
}
struct Mib(*mut MIB_IPFORWARD_TABLE2);
impl Drop for Mib {
    fn drop(&mut self) {
        unsafe {
            FreeMibTable(self.0.cast());
        }
    }
}
struct WlanHandle(HANDLE);
impl Drop for WlanHandle {
    fn drop(&mut self) {
        unsafe {
            WlanCloseHandle(self.0, null());
        }
    }
}
struct WlanMemory(*mut c_void);
impl Drop for WlanMemory {
    fn drop(&mut self) {
        unsafe {
            WlanFreeMemory(self.0);
        }
    }
}

// Strings are OS-owned NUL-terminated buffers valid for the duration of the call.
unsafe fn wide(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let len = (0..32768).take_while(|i| *p.add(*i) != 0).count();
    String::from_utf16_lossy(std::slice::from_raw_parts(p, len))
}
unsafe fn socket_ip(p: *const SOCKADDR) -> Option<IpAddr> {
    if p.is_null() {
        return None;
    }
    match (*p).sa_family {
        AF_INET => Some(IpAddr::V4(Ipv4Addr::from(
            (*p.cast::<SOCKADDR_IN>())
                .sin_addr
                .S_un
                .S_addr
                .to_ne_bytes(),
        ))),
        AF_INET6 => Some(IpAddr::V6(Ipv6Addr::from(
            (*p.cast::<SOCKADDR_IN6>()).sin6_addr.u.Byte,
        ))),
        _ => None,
    }
}
unsafe fn inet_ip(p: &SOCKADDR_INET) -> Option<IpAddr> {
    socket_ip((p as *const SOCKADDR_INET).cast())
}

pub fn snapshot() -> Result<Snapshot> {
    // SAFETY: the buffer is u64-aligned, at least the requested byte length, and
    // retained while walking the linked lists filled by GetAdaptersAddresses.
    unsafe {
        let mut size = 15000u32;
        for _ in 0..4 {
            if size > 4 * 1024 * 1024 {
                return Err(Error::Platform("adapter buffer exceeds limit".into()));
            }
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let ptr = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
            let status = GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_INCLUDE_GATEWAYS,
                null(),
                ptr,
                &mut size,
            );
            if status == 111 {
                continue;
            }
            if status != 0 {
                return Err(Error::Platform(format!(
                    "GetAdaptersAddresses error {status}"
                )));
            }
            let mut snapshot = Snapshot::default();
            let mut current = ptr;
            while !current.is_null() && snapshot.adapters.len() < 256 {
                let a = &*current;
                let description = wide(a.Description);
                let lowered = description.to_lowercase();
                let kind = match a.IfType {
                    71 => "wifi",
                    6 => "ethernet",
                    24 => "loopback",
                    131 => "tunnel",
                    _ => "other",
                };
                let mut item = Adapter {
                    name: wide(a.FriendlyName),
                    description,
                    index: a.Anonymous1.Anonymous.IfIndex,
                    kind: kind.into(),
                    up: a.OperStatus == 1,
                    virtual_adapter: matches!(a.IfType, 131 | 53)
                        || [
                            "vpn",
                            "tun",
                            "tap-",
                            "wireguard",
                            "wintun",
                            "hyper-v",
                            "virtual",
                            "tailscale",
                            "zerotier",
                        ]
                        .iter()
                        .any(|s| lowered.contains(s)),
                    dhcp: a.Anonymous2.Flags & 4 != 0,
                    mac: a.PhysicalAddress[..a.PhysicalAddressLength.min(8) as usize]
                        .iter()
                        .map(|b| format!("{b:02X}"))
                        .collect::<Vec<_>>()
                        .join(":"),
                    link_mbps: a.TransmitLinkSpeed / 1_000_000,
                    ..Adapter::default()
                };
                let mut u = a.FirstUnicastAddress;
                while !u.is_null() && item.addresses.len() < 128 {
                    if let Some(ip) = socket_ip((*u).Address.lpSockaddr) {
                        item.addresses.push(ip.to_string());
                    }
                    u = (*u).Next;
                }
                let mut d = a.FirstDnsServerAddress;
                while !d.is_null() && item.dns_servers.len() < 64 {
                    if let Some(ip) = socket_ip((*d).Address.lpSockaddr) {
                        item.dns_servers.push(ip.to_string());
                    }
                    d = (*d).Next;
                }
                let mut g = a.FirstGatewayAddress;
                while !g.is_null() && item.gateways.len() < 32 {
                    if let Some(ip) = socket_ip((*g).Address.lpSockaddr) {
                        item.gateways.push(ip.to_string());
                    }
                    g = (*g).Next;
                }
                snapshot.adapters.push(item);
                current = a.Next;
            }
            match routes() {
                Ok(r) => snapshot.routes = r,
                Err(e) => snapshot.warnings.push(e.to_string()),
            }
            snapshot.proxy = proxy();
            wifi(&mut snapshot);
            return Ok(snapshot);
        }
    }
    Err(Error::Platform(
        "adapter list changed repeatedly; retry".into(),
    ))
}
unsafe fn routes() -> Result<Vec<Route>> {
    let mut p = null_mut();
    let status = GetIpForwardTable2(AF_UNSPEC, &mut p);
    if status != 0 || p.is_null() {
        return Err(Error::Platform(format!(
            "GetIpForwardTable2 error {status}"
        )));
    }
    let table = Mib(p);
    let len = (*table.0).NumEntries as usize;
    if len > 65536 {
        return Err(Error::Platform("route table exceeds limit".into()));
    }
    let rows = std::slice::from_raw_parts((*table.0).Table.as_ptr(), len);
    Ok(rows
        .iter()
        .filter(|r| r.DestinationPrefix.PrefixLength == 0)
        .filter_map(|r| {
            let ip = inet_ip(&r.NextHop)?;
            Some(Route {
                family: if ip.is_ipv4() { "IPv4" } else { "IPv6" }.into(),
                interface_index: r.InterfaceIndex,
                next_hop: ip.to_string(),
                metric: r.Metric,
            })
        })
        .collect())
}
unsafe fn proxy() -> ProxyConfig {
    let mut result = ProxyConfig::default();
    let mut ie: WINHTTP_CURRENT_USER_IE_PROXY_CONFIG = std::mem::zeroed();
    if WinHttpGetIEProxyConfigForCurrentUser(&mut ie) != 0 {
        result.auto_detect = ie.fAutoDetect != 0;
        if !ie.lpszProxy.is_null() {
            result.system_enabled = true;
            result.system_server = Some(wide(ie.lpszProxy));
        }
        if !ie.lpszAutoConfigUrl.is_null() {
            result.pac_url = Some(wide(ie.lpszAutoConfigUrl));
        }
        for p in [ie.lpszProxy, ie.lpszAutoConfigUrl, ie.lpszProxyBypass] {
            if !p.is_null() {
                GlobalFree(p.cast());
            }
        }
    } else {
        result
            .notes
            .push(format!("Current-user proxy API error {}", GetLastError()));
    }
    let mut info: WINHTTP_PROXY_INFO = std::mem::zeroed();
    if WinHttpGetDefaultProxyConfiguration(&mut info) != 0 {
        if !info.lpszProxy.is_null() {
            result.winhttp_server = Some(wide(info.lpszProxy));
        }
        for p in [info.lpszProxy, info.lpszProxyBypass] {
            if !p.is_null() {
                GlobalFree(p.cast());
            }
        }
    } else {
        result
            .notes
            .push(format!("WinHTTP proxy API error {}", GetLastError()));
    }
    result.notes.push("PAC/WPAD scripts are reported, never downloaded or executed. Per-application proxy state is not inspected.".into());
    result
}
unsafe fn wifi(snapshot: &mut Snapshot) {
    let mut h = null_mut();
    let mut version = 0;
    let status = WlanOpenHandle(2, null(), &mut version, &mut h);
    if status != 0 {
        snapshot.warnings.push(format!("WLAN unavailable (error {status}); Wi-Fi signal/SSID unknown. Windows location permission may be required."));
        return;
    }
    let handle = WlanHandle(h);
    let mut list = null_mut();
    if WlanEnumInterfaces(handle.0, null(), &mut list) != 0 || list.is_null() {
        return;
    }
    let _list_mem = WlanMemory(list.cast());
    for info in std::slice::from_raw_parts(
        (*list).InterfaceInfo.as_ptr(),
        (*list).dwNumberOfItems.min(256) as usize,
    ) {
        let mut size = 0;
        let mut data = null_mut();
        let status = WlanQueryInterface(
            handle.0,
            &info.InterfaceGuid,
            wlan_intf_opcode_current_connection,
            null(),
            &mut size,
            &mut data,
            null_mut(),
        );
        if status != 0 {
            snapshot.warnings.push(format!("WLAN current connection unavailable (error {status}); location permission may be required."));
            continue;
        }
        let _data_mem = WlanMemory(data);
        if data.is_null() || (size as usize) < std::mem::size_of::<WLAN_CONNECTION_ATTRIBUTES>() {
            continue;
        }
        let c = &*data.cast::<WLAN_CONNECTION_ATTRIBUTES>();
        let descr = String::from_utf16_lossy(
            &info.strInterfaceDescription[..info
                .strInterfaceDescription
                .iter()
                .position(|v| *v == 0)
                .unwrap_or(256)],
        );
        if let Some(a) = snapshot
            .adapters
            .iter_mut()
            .find(|a| a.kind == "wifi" && a.description == descr)
        {
            let assoc = &c.wlanAssociationAttributes;
            a.signal_percent = Some(assoc.wlanSignalQuality);
            a.ssid = Some(
                String::from_utf8_lossy(
                    &assoc.dot11Ssid.ucSSID[..assoc.dot11Ssid.uSSIDLength.min(32) as usize],
                )
                .into_owned(),
            );
        }
    }
}

pub fn ping(ip: IpAddr, ttl: u8, timeout_ms: u32) -> Result<PingReply> {
    // SAFETY: synchronous native requests finish before owned request/reply
    // storage and the RAII handle are dropped; reply storage is 8-byte aligned.
    unsafe {
        let h = if ip.is_ipv4() {
            IcmpCreateFile()
        } else {
            Icmp6CreateFile()
        };
        if h == INVALID_HANDLE_VALUE || h.is_null() {
            return Err(Error::Platform(format!(
                "ICMP handle error {}",
                GetLastError()
            )));
        }
        let handle = IcmpHandle(h);
        let request = *b"HoriNet Doctor";
        let mut options: IP_OPTION_INFORMATION = std::mem::zeroed();
        options.Ttl = ttl;
        let mut reply = vec![0u64; 128];
        let count = match ip {
            IpAddr::V4(ip) => IcmpSendEcho(
                handle.0,
                u32::from_ne_bytes(ip.octets()),
                request.as_ptr().cast(),
                request.len() as u16,
                &options,
                reply.as_mut_ptr().cast(),
                (reply.len() * 8) as u32,
                timeout_ms,
            ),
            IpAddr::V6(ip) => {
                let mut src: SOCKADDR_IN6 = std::mem::zeroed();
                src.sin6_family = AF_INET6;
                let mut dst = src;
                dst.sin6_addr.u.Byte = ip.octets();
                Icmp6SendEcho2(
                    handle.0,
                    null_mut(),
                    None,
                    null(),
                    &src,
                    &dst,
                    request.as_ptr().cast(),
                    request.len() as u16,
                    &options,
                    reply.as_mut_ptr().cast(),
                    (reply.len() * 8) as u32,
                    timeout_ms,
                )
            }
        };
        if count == 0 {
            let code = GetLastError();
            return Ok(PingReply {
                address: None,
                rtt_ms: None,
                status_code: code,
                reached: false,
                message: format!("No ICMP reply (Windows status {code}); ICMP may be filtered"),
            });
        }
        let (address, status, rtt) = if ip.is_ipv4() {
            let r = &*reply.as_ptr().cast::<ICMP_ECHO_REPLY>();
            (
                Ipv4Addr::from(r.Address.to_ne_bytes()).to_string(),
                r.Status,
                r.RoundTripTime,
            )
        } else {
            let r = &*reply.as_ptr().cast::<ICMPV6_ECHO_REPLY_LH>();
            (
                Ipv6Addr::from(r.Address.sin6_addr.map(u16::from_be)).to_string(),
                r.Status,
                r.RoundTripTime,
            )
        };
        let answered = status == 0 || status == 11013;
        Ok(PingReply {
            address: answered.then_some(address),
            rtt_ms: answered.then_some(rtt as f64),
            status_code: status,
            reached: status == 0,
            message: match status {
                0 => "Echo reply",
                11013 => "TTL / hop limit exceeded",
                _ => "ICMP error / no reply; not proof of outage",
            }
            .into(),
        })
    }
}
