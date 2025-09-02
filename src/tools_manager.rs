// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

pub fn get_ffmpeg_url(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("https://example.com/ffmpeg-windows.exe"),
        "macos-arm" => Some("https://www.osxexperts.net/ffmpeg80arm.zip"),
        "macos-x86" => Some("https://www.osxexperts.net/ffmpeg71intel.zip"),
        "linux" => Some("https://example.com/ffmpeg-linux"),
        _ => None,
    }
}

pub fn get_ffmpeg_sha256(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some(""),
        "macos-arm" => Some("77d2c853f431318d55ec02676d9b2f185ebfdddb9f7677a251fbe453affe025a"),
        "macos-x86" => Some(""),
        "linux" => Some(""),
        _ => None,
    }
}

pub fn get_ffprobe_url(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("https://example.com/ffprobe-windows.exe"),
        "macos-arm" => Some("https://www.osxexperts.net/ffprobe80arm.zip"),
        "macos-x86" => Some("https://www.osxexperts.net/ffprobe71intel.zip"),
        "linux" => Some("https://example.com/ffprobe-linux"),
        _ => None,
    }
}

pub fn get_ffprobe_sha256(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some(""),
        "macos-arm" => Some("babf170e86bd6b0b2fefee5fa56f57721b0acb98ad2794b095d8030b02857dfe"),
        "macos-x86" => Some(""),
        "linux" => Some(""),
        _ => None,
    }
}

pub fn get_whisper_cli_url(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("https://example.com/ffprobe-windows.exe"),
        "macos-arm" => Some("https://autotranscript.s3.us-east-1.amazonaws.com/binaries/whisper-cli"),
        "macos-x86" => Some("https://www.osxexperts.net/ffprobe71intel.zip"),
        "linux" => Some("https://example.com/ffprobe-linux"),
        _ => None,
    }
}

pub fn get_whisper_cli_sha256(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some(""),
        "macos-arm" => Some("f2aa391fb826ae37fcf39911280985d8776ff9c77ff7c371ab878d47c20d11df"),
        "macos-x86" => Some(""),
        "linux" => Some(""),
        _ => None,
    }
}

pub fn binaries_directory(tool: &str) -> std::path::PathBuf {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home_dir).join(".atci").join(tool)
}

#[derive(Debug, serde::Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub platform: String,
    pub downloaded: bool,
    pub downloaded_path: String,
    pub system_available: bool,
    pub system_path: Option<String>,
    pub current_path: String,
}

pub fn list_tools() -> Vec<ToolInfo> {
    let platform = detect_platform();
    let tools = ["ffmpeg", "ffprobe", "whisper-cli"];

    let cfg: crate::AtciConfig = crate::config::load_config_or_default();
    
    tools.iter().map(|&tool| {
        let downloaded_path = get_downloaded_path(tool);
        let system_path = find_in_system_path(tool);
        
        ToolInfo {
            name: tool.to_string(),
            platform: platform.clone(),
            downloaded: std::path::Path::new(&downloaded_path).exists(),
            downloaded_path: downloaded_path.clone(),
            system_available: system_path.is_some(),
            system_path: system_path.clone(),
            current_path: get_current_path(tool, &cfg),
        }
    }).collect()
}

fn detect_platform() -> String {
    if cfg!(target_os = "windows") {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "macos-arm".to_string()
        } else {
            "macos-x86".to_string()
        }
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        "unknown".to_string()
    }
}

fn get_downloaded_path(tool: &str) -> String {
    let binaries_dir = binaries_directory(tool);
    let extension = if cfg!(target_os = "windows") { ".exe" } else { "" };
    binaries_dir.join(format!("{}{}", tool, extension)).to_string_lossy().to_string()
}

fn find_in_system_path(tool: &str) -> Option<String> {
    which::which(tool).ok().map(|path| path.to_string_lossy().to_string())
}

fn get_current_path(tool: &str, cfg: &crate::AtciConfig) -> String {
    if tool == "whisper-cli" {
        cfg.whispercli_path.clone()
    } else if tool == "ffmpeg" {
        cfg.ffmpeg_path.clone()
    } else if tool == "ffprobe" {
        cfg.ffprobe_path.clone()
    } else {
        "not found".to_string()
    }
}

pub fn download_tool(tool: &str) -> Result<String, Box<dyn std::error::Error>> {
    let platform = detect_platform();
    
    let url = match tool {
        "ffmpeg" => get_ffmpeg_url(&platform),
        "ffprobe" => get_ffprobe_url(&platform),
        "whisper-cli" => get_whisper_cli_url(&platform),
        _ => return Err(format!("Unknown tool: {}", tool).into()),
    };
    println!("Downloading tool: {} from {}", tool, url.unwrap());

    let url = url.ok_or(format!("No download URL available for {} on {}", tool, platform))?;
    
    let binaries_dir = binaries_directory(tool);
    std::fs::create_dir_all(&binaries_dir)?;
    
    let response = reqwest::blocking::get(url)?;
    let bytes = response.bytes()?;
    
    // Handle whisper-cli separately as it's a direct binary download
    if tool == "whisper-cli" && platform == "macos-arm" {
        let extension = if cfg!(target_os = "windows") { ".exe" } else { "" };
        let output_path = binaries_dir.join(format!("{}{}", tool, extension));
        
        std::fs::write(&output_path, &bytes)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&output_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&output_path, permissions)?;
        }
        
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = handle_macos_quarantine(&output_path.to_string_lossy(), &platform) {
                eprintln!("Warning: Failed to handle macOS quarantine: {}", e);
            }
        }
        
        // Verify SHA256 hash
        if let Some(expected_hash) = get_tool_sha256(tool, &platform) {
            match verify_sha256(&output_path.to_string_lossy(), expected_hash) {
                Ok(true) => println!("SHA256 verification successful for {}", tool),
                Ok(false) => {
                    std::fs::remove_file(&output_path)?;
                    return Err(format!("SHA256 hash verification failed for {}", tool).into());
                }
                Err(e) => {
                    eprintln!("Warning: SHA256 verification error for {}: {}", tool, e);
                }
            }
        } else {
            eprintln!("Warning: No expected SHA256 hash found for {} on {}", tool, platform);
        }
        
        // Create GPL license file for ffmpeg and ffprobe
        if tool == "ffmpeg" || tool == "ffprobe" {
            create_gpl_license_file(&binaries_dir)?;
            create_compiling_file(&binaries_dir)?;
        }
        
        return Ok(output_path.to_string_lossy().to_string());
    }
    
    // Handle zip archives for other tools
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name = file.name();
        
        if file_name.contains(tool) && !file_name.ends_with('/') {
            let extension = if cfg!(target_os = "windows") { ".exe" } else { "" };
            let output_path = binaries_dir.join(format!("{}{}", tool, extension));
            
            let mut output_file = std::fs::File::create(&output_path)?;
            std::io::copy(&mut file, &mut output_file)?;
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(&output_path)?;
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o755);
                std::fs::set_permissions(&output_path, permissions)?;
            }
            
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = handle_macos_quarantine(&output_path.to_string_lossy(), &platform) {
                    eprintln!("Warning: Failed to handle macOS quarantine: {}", e);
                }
            }
            
            // Verify SHA256 hash
            if let Some(expected_hash) = get_tool_sha256(tool, &platform) {
                match verify_sha256(&output_path.to_string_lossy(), expected_hash) {
                    Ok(true) => println!("SHA256 verification successful for {}", tool),
                    Ok(false) => {
                        std::fs::remove_file(&output_path)?;
                        return Err(format!("SHA256 hash verification failed for {}", tool).into());
                    }
                    Err(e) => {
                        eprintln!("Warning: SHA256 verification error for {}: {}", tool, e);
                    }
                }
            } else {
                eprintln!("Warning: No expected SHA256 hash found for {} on {}", tool, platform);
            }
            
            // Create GPL license file for ffmpeg and ffprobe
            if tool == "ffmpeg" || tool == "ffprobe" {
                create_gpl_license_file(&binaries_dir)?;
                create_compiling_file(&binaries_dir)?;
            }
            
            return Ok(output_path.to_string_lossy().to_string());
        }
    }
    
    Err(format!("Could not find {} binary in the downloaded archive", tool).into())
}

fn create_gpl_license_file(binaries_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let license_path = binaries_dir.join("COPYING.GPLv2");
    let license_text = r#"                    GNU GENERAL PUBLIC LICENSE
                       Version 2, June 1991

 Copyright (C) 1989, 1991 Free Software Foundation, Inc.,
 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA
 Everyone is permitted to copy and distribute verbatim copies
 of this license document, but changing it is not allowed.

                            Preamble

  The licenses for most software are designed to take away your
freedom to share and change it.  By contrast, the GNU General Public
License is intended to guarantee your freedom to share and change free
software--to make sure the software is free for all its users.  This
General Public License applies to most of the Free Software
Foundation's software and to any other program whose authors commit to
using it.  (Some other Free Software Foundation software is covered by
the GNU Lesser General Public License instead.)  You can apply it to
your programs, too.

  When we speak of free software, we are referring to freedom, not
price.  Our General Public Licenses are designed to make sure that you
have the freedom to distribute copies of free software (and charge for
this service if you wish), that you receive source code or can get it
if you want it, that you can change the software or use pieces of it
in new free programs; and that you know you can do these things.

  To protect your rights, we need to make restrictions that forbid
anyone to deny you these rights or to ask you to surrender the rights.
These restrictions translate to certain responsibilities for you if you
distribute copies of the software, or if you modify it.

  For example, if you distribute copies of such a program, whether
gratis or for a fee, you must give the recipients all the rights that
you have.  You must make sure that they, too, receive or can get the
source code.  And you must show them these terms so they know their
rights.

  We protect your rights with two steps: (1) copyright the software, and
(2) offer you this license which gives you legal permission to copy,
distribute and/or modify the software.

  Also, for each author's protection and ours, we want to make certain
that everyone understands that there is no warranty for this free
software.  If the software is modified by someone else and passed on, we
want its recipients to know that what they have is not the original, so
that any problems introduced by others will not reflect on the original
authors' reputations.

  Finally, any free program is threatened constantly by software
patents.  We wish to avoid the danger that redistributors of a free
program will individually obtain patent licenses, in effect making the
program proprietary.  To prevent this, we have made it clear that any
patent must be licensed for everyone's free use or not licensed at all.

  The precise terms and conditions for copying, distribution and
modification follow.

                    GNU GENERAL PUBLIC LICENSE
   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION

  0. This License applies to any program or other work which contains
a notice placed by the copyright holder saying it may be distributed
under the terms of this General Public License.  The "Program", below,
refers to any such program or work, and a "work based on the Program"
means either the Program or any derivative work under copyright law:
that is to say, a work containing the Program or a portion of it,
either verbatim or with modifications and/or translated into another
language.  (Hereinafter, translation is included without limitation in
the term "modification".)  Each licensee is addressed as "you".

Activities other than copying, distribution and modification are not
covered by this License; they are outside its scope.  The act of
running the Program is not restricted, and the output from the Program
is covered only if its contents constitute a work based on the
Program (independent of having been made by running the Program).
Whether that is true depends on what the Program does.

  1. You may copy and distribute verbatim copies of the Program's
source code as you receive it, in any medium, provided that you
conspicuously and appropriately publish on each copy an appropriate
copyright notice and disclaimer of warranty; keep intact all the
notices that refer to this License and to the absence of any warranty;
and give any other recipients of the Program a copy of this License
along with the Program.

You may charge a fee for the physical act of transferring a copy, and
you may at your option offer warranty protection in exchange for a fee.

  2. You may modify your copy or copies of the Program or any portion
of it, thus forming a work based on the Program, and copy and
distribute such modifications or work under the terms of Section 1
above, provided that you also meet all of these conditions:

    a) You must cause the modified files to carry prominent notices
    stating that you changed the files and the date of any change.

    b) You must cause any work that you distribute or publish, that in
    whole or in part contains or is derived from the Program or any
    part thereof, to be licensed as a whole at no charge to all third
    parties under the terms of this License.

    c) If the modified program normally reads commands interactively
    when run, you must cause it, when started running for such
    interactive use in the most ordinary way, to print or display an
    announcement including an appropriate copyright notice and a
    notice that there is no warranty (or else, saying that you provide
    a warranty) and that users may redistribute the program under
    these conditions, and telling the user how to view a copy of this
    License.  (Exception: if the Program itself is interactive but
    does not normally print such an announcement, your work based on
    the Program is not required to print an announcement.)

These requirements apply to the modified work as a whole.  If
identifiable sections of that work are not derived from the Program,
and can be reasonably considered independent and separate works in
themselves, then this License, and its terms, do not apply to those
sections when you distribute them as separate works.  But when you
distribute the same sections as part of a whole which is a work based
on the Program, the distribution of the whole must be on the terms of
this License, whose permissions for other licensees extend to the
entire whole, and thus to each and every part regardless of who wrote it.

Thus, it is not the intent of this section to claim rights or contest
your rights to work written entirely by you; rather, the intent is to
exercise the right to control the distribution of derivative or
collective works based on the Program.

In addition, mere aggregation of another work not based on the Program
with the Program (or with a work based on the Program) on a volume of
a storage or distribution medium does not bring the other work under
the scope of this License.

  3. You may copy and distribute the Program (or a work based on it,
under Section 2) in object code or executable form under the terms of
Sections 1 and 2 above provided that you also do one of the following:

    a) Accompany it with the complete corresponding machine-readable
    source code, which must be distributed under the terms of Sections
    1 and 2 above on a medium customarily used for software interchange; or,

    b) Accompany it with a written offer, valid for at least three
    years, to give any third party, for a charge no more than your
    cost of physically performing source distribution, a complete
    machine-readable copy of the corresponding source code, to be
    distributed under the terms of Sections 1 and 2 above on a medium
    customarily used for software interchange; or,

    c) Accompany it with the information you received as to the offer
    to distribute corresponding source code.  (This alternative is
    allowed only for noncommercial distribution and only if you
    received the program in object code or executable form with such
    an offer, in accord with Subsection b above.)

The source code for a work means the preferred form of the work for
making modifications to it.  For an executable work, complete source
code means all the source code for all modules it contains, plus any
associated interface definition files, plus the scripts used to
control compilation and installation of the executable.  However, as a
special exception, the source code distributed need not include
anything that is normally distributed (in either source or binary
form) with the major components (compiler, kernel, and so on) of the
operating system on which the executable runs, unless that component
itself accompanies the executable.

If distribution of executable or object code is made by offering
access to copy from a designated place, then offering equivalent
access to copy the source code from the same place counts as
distribution of the source code, even though third parties are not
compelled to copy the source along with the object code.

  4. You may not copy, modify, sublicense, or distribute the Program
except as expressly provided under this License.  Any attempt
otherwise to copy, modify, sublicense or distribute the Program is
void, and will automatically terminate your rights under this License.
However, parties who have received copies, or rights, from you under
this License will not have their licenses terminated so long as such
parties remain in full compliance.

  5. You are not required to accept this License, since you have not
signed it.  However, nothing else grants you permission to modify or
distribute the Program or its derivative works.  These actions are
prohibited by law if you do not accept this License.  Therefore, by
modifying or distributing the Program (or any work based on the
Program), you indicate your acceptance of this License to do so, and
all its terms and conditions for copying, distributing or modifying
the Program or works based on it.

  6. Each time you redistribute the Program (or any work based on the
Program), the recipient automatically receives a license from the
original licensor to copy, distribute or modify the Program subject to
these terms and conditions.  You may not impose any further
restrictions on the recipients' exercise of the rights granted herein.
You are not responsible for enforcing compliance by third parties to
this License.

  7. If, as a consequence of a court judgment or allegation of patent
infringement or for any other reason (not limited to patent issues),
conditions are imposed on you (whether by court order, agreement or
otherwise) that contradict the conditions of this License, they do not
excuse you from the conditions of this License.  If you cannot
distribute so as to satisfy simultaneously your obligations under this
License and any other pertinent obligations, then as a consequence you
may not distribute the Program at all.  For example, if a patent
license would not permit royalty-free redistribution of the Program by
all those who receive copies directly or indirectly through you, then
the only way you could satisfy both it and this License would be to
refrain entirely from distribution of the Program.

If any portion of this section is held invalid or unenforceable under
any particular circumstance, the balance of the section is intended to
apply and the section as a whole is intended to apply in other
circumstances.

It is not the purpose of this section to induce you to infringe any
patents or other property right claims or to contest validity of any
such claims; this section has the sole purpose of protecting the
integrity of the free software distribution system, which is
implemented by public license practices.  Many people have made
generous contributions to the wide range of software distributed
through that system in reliance on consistent application of that
system; it is up to the author/donor to decide if he or she is willing
to distribute software through any other system and a licensee cannot
impose that choice.

This section is intended to make thoroughly clear what is believed to
be a consequence of the rest of this License.

  8. If the distribution and/or use of the Program is restricted in
certain countries either by patents or by copyrighted interfaces, the
original copyright holder who places the Program under this License
may add an explicit geographical distribution limitation excluding
those countries, so that distribution is permitted only in or among
countries not thus excluded.  In such case, this License incorporates
the limitation as if written in the body of this License.

  9. The Free Software Foundation may publish revised and/or new versions
of the General Public License from time to time.  Such new versions will
be similar in spirit to the present version, but may differ in detail to
address new problems or concerns.

Each version is given a distinguishing version number.  If the Program
specifies a version number of this License which applies to it and "any
later version", you have the option of following the terms and conditions
either of that version or of any later version published by the Free
Software Foundation.  If the Program does not specify a version number of
this License, you may choose any version ever published by the Free Software
Foundation.

  10. If you wish to incorporate parts of the Program into other free
programs whose distribution conditions are different, write to the author
to ask for permission.  For software which is copyrighted by the Free
Software Foundation, write to the Free Software Foundation; we sometimes
make exceptions for this.  Our decision will be guided by the two goals
of preserving the free status of all derivatives of our free software and
of promoting the sharing and reuse of software generally.

                            NO WARRANTY

  11. BECAUSE THE PROGRAM IS LICENSED FREE OF CHARGE, THERE IS NO WARRANTY
FOR THE PROGRAM, TO THE EXTENT PERMITTED BY APPLICABLE LAW.  EXCEPT WHEN
OTHERWISE STATED IN WRITING THE COPYRIGHT HOLDERS AND/OR OTHER PARTIES
PROVIDE THE PROGRAM "AS IS" WITHOUT WARRANTY OF ANY KIND, EITHER EXPRESSED
OR IMPLIED, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE.  THE ENTIRE RISK AS
TO THE QUALITY AND PERFORMANCE OF THE PROGRAM IS WITH YOU.  SHOULD THE
PROGRAM PROVE DEFECTIVE, YOU ASSUME THE COST OF ALL NECESSARY SERVICING,
REPAIR OR CORRECTION.

  12. IN NO EVENT UNLESS REQUIRED BY APPLICABLE LAW OR AGREED TO IN WRITING
WILL ANY COPYRIGHT HOLDER, OR ANY OTHER PARTY WHO MAY MODIFY AND/OR
REDISTRIBUTE THE PROGRAM AS PERMITTED ABOVE, BE LIABLE TO YOU FOR DAMAGES,
INCLUDING ANY GENERAL, SPECIAL, INCIDENTAL OR CONSEQUENTIAL DAMAGES ARISING
OUT OF THE USE OR INABILITY TO USE THE PROGRAM (INCLUDING BUT NOT LIMITED
TO LOSS OF DATA OR DATA BEING RENDERED INACCURATE OR LOSSES SUSTAINED BY
YOU OR THIRD PARTIES OR A FAILURE OF THE PROGRAM TO OPERATE WITH ANY OTHER
PROGRAMS), EVEN IF SUCH HOLDER OR OTHER PARTY HAS BEEN ADVISED OF THE
POSSIBILITY OF SUCH DAMAGES.

                     END OF TERMS AND CONDITIONS

            How to Apply These Terms to Your New Programs

  If you develop a new program, and you want it to be of the greatest
possible use to the public, the best way to achieve this is to make it
free software which everyone can redistribute and change under these terms.

  To do so, attach the following notices to the program.  It is safest
to attach them to the start of each source file to most effectively
convey the exclusion of warranty; and each file should have at least
the "copyright" line and a pointer to where the full notice is found.

    <one line to give the program's name and a brief idea of what it does.>
    Copyright (C) <year>  <name of author>

    This program is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation; either version 2 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License along
    with this program; if not, write to the Free Software Foundation, Inc.,
    51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

Also add information on how to contact you by electronic and paper mail.

If the program is interactive, make it output a short notice like this
when it starts in an interactive mode:

    Gnomovision version 69, Copyright (C) year name of author
    Gnomovision comes with ABSOLUTELY NO WARRANTY; for details type `show w'.
    This is free software, and you are welcome to redistribute it
    under certain conditions; type `show c' for details.

The hypothetical commands `show w' and `show c' should show the appropriate
parts of the General Public License.  Of course, the commands you use may
be called something other than `show w' and `show c'; they could even be
mouse-clicks or menu items--whatever suits your program.

You should also get your employer (if you work as a programmer) or your
school, if any, to sign a "copyright disclaimer" for the program, if
necessary.  Here is a sample; alter the names:

  Yoyodyne, Inc., hereby disclaims all copyright interest in the program
  `Gnomovision' (which makes passes at compilers) written by James Hacker.

  <signature of Ty Coon>, 1 April 1989
  Ty Coon, President of Vice

This General Public License does not permit incorporating your program into
proprietary programs.  If your program is a subroutine library, you may
consider it more useful to permit linking proprietary applications with the
library.  If this is what you want to do, use the GNU Lesser General
Public License instead of this License."#;
    
    std::fs::write(license_path, license_text)?;
    Ok(())
}

fn create_compiling_file(binaries_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let compiling_path = binaries_dir.join("COMPILING");
    let compiling_text = r#"instructions based on https://www.osxexperts.net/

gather dependencies
- x264           git clone https://code.videolan.org/videolan/x264.git
- x265           https://bitbucket.org/multicoreware/x265/downloads/
- cmake   
- enca.          https://dl.cihar.com/enca/enca-1.19.tar.gz
- expat          https://github.com/libexpat/libexpat/releases/download/R_2_2_10/expat-2.2.10.tar.gz
- lame           git clone https://github.com/rbrito/lame.git
- fribidi          https://github.com/fribidi/fribidi/releases/download/
- freetype      https://download.savannah.gnu.org/releases/freetype/
- fontconfig    https://www.freedesktop.org/software/fontconfig/release/
- libiconv.      https://ftp.gnu.org/pub/gnu/libiconv/libiconv-1.16.tar.gz
- libass          https://github.com/libass/libass/releases/download/
- nasm.         https://www.nasm.us/pub/nasm/releasebuilds/2.15.05/nasm-2.15.05.tar.gz
- yasm          http://www.tortall.net/projects/yasm/releases/
- pkg-config  https://pkg-config.freedesktop.org/releases/
- ffmpeg         git clone git://git.ffmpeg.org/ffmpeg.git

as well as xcode

compile everything with this bash script:

echo '♻️ ' Create Ramdisk

if df | grep Ramdisk > /dev/null ; then tput bold ; echo ; echo ⏏ Eject Ramdisk ; tput sgr0 ; fi

if df | grep Ramdisk > /dev/null ; then diskutil eject Ramdisk ; sleep 1 ; fi

DISK_ID=$(hdid -nomount ram://70000000)

newfs_hfs -v tempdisk ${DISK_ID}

diskutil mount ${DISK_ID}

sleep 1

SOURCE="/Volumes/tempdisk/sw"

COMPILED="/Volumes/tempdisk/compile"

mkdir ${SOURCE}

mkdir ${COMPILED}

export PATH=${SOURCE}/bin:$PATH

export CC=clang && export PKG_CONFIG_PATH="${SOURCE}/lib/pkgconfig"

# export MACOSX_DEPLOYMENT_TARGET=10.9

# set -o errexit

#
# ask user to copy all files to ramdisk
#

echo
echo Copy all files to ramdisk tempdisk/compile folder and press a key when ready
read -s

# echo '♻️ ' Start compiling YASM

#
# compile YASM
#

#cd ${COMPILED}

#cd yasm-1.3.0

#./configure --prefix=${SOURCE}

#make -j 10

#make install

#sleep 1


echo '♻️ ' Start compiling NASM

#
# compile NASM
#

cd ${COMPILED}

cd nasm-2.16.01

./configure --prefix=${SOURCE}

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling PKG

#
# compile PKG
#

cd ${COMPILED}

cd pkg-config-0.29.2

export LDFLAGS="-framework Foundation -framework Cocoa"

./configure --prefix=${SOURCE} --with-pc-path=${SOURCE}/lib/pkgconfig --with-internal-glib --disable-shared --enable-static

make -j 10

make install

unset LDFLAGS

sleep 1

#
# compile XZ
#
echo '♻️ ' Start compiling ZX

cd ${COMPILED}

cd xz

./configure --prefix=${SOURCE} --disable-shared --enable-static --disable-docs --disable-examples

make -j 10

make install

echo '♻️ ' Start compiling ZLIB

#
# ZLIB
#


cd ${COMPILED}

cd zlib-1.2.13

./configure --prefix=${SOURCE} --static

make -j 10

make install

rm ${SOURCE}/lib/libz.so*

rm ${SOURCE}/lib/libz.*

echo '♻️ ' Start compiling CMAKE

#
# compile CMAKE
#

cd ${COMPILED}

cd cmake-3.25.1

./configure --prefix=${SOURCE}

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling xml2

#
# compile libxml2
#

cd ${COMPILED}

cd libxml2-v2.10.3

./autogen.sh

./configure --prefix=${SOURCE} --disable-shared --enable-static --without-python

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling Lame

#
# compile Lame
#

cd ${COMPILED}

cd lame

./configure --prefix=${SOURCE} --disable-shared --enable-static

make -j 10

make install

echo '♻️ ' Start compiling X264

#
# x264
#

cd ${COMPILED}

cd x264

./configure --prefix=${SOURCE} --disable-shared --enable-static --enable-pic

make -j 10

make install

make install-lib-static

# echo
# echo continue
# read -s

sleep 1

echo '♻️ ' Start compiling X265

#
# x265
#

rm -f ${SOURCE}/include/x265*.h 2>/dev/null

rm -f ${SOURCE}/lib/libx265.a 2>/dev/null

echo '♻️ ' X265 12bit

cd ${COMPILED}

cd /Volumes/tempdisk/compile/x265Neon/source

cmake -DCMAKE_INSTALL_PREFIX:PATH=${SOURCE} -DHIGH_BIT_DEPTH=ON -DMAIN12=ON -DENABLE_SHARED=NO -DEXPORT_C_API=NO -DENABLE_CLI=OFF .

make

mv libx265.a libx265_main12.a

make clean-generated

rm CMakeCache.txt

echo '♻️ ' X265 10bit

cd ${COMPILED}

cd /Volumes/tempdisk/compile/x265Neon/source

cmake -DCMAKE_INSTALL_PREFIX:PATH=${SOURCE} -DHIGH_BIT_DEPTH=ON -DMAIN10=ON -DENABLE_HDR10_PLUS=ON -DENABLE_SHARED=NO -DEXPORT_C_API=NO -DENABLE_CLI=OFF .

make clean

make

mv libx265.a libx265_main10.a

make clean-generated && rm CMakeCache.txt

echo '♻️ ' X265 full

cd ${COMPILED}

cd /Volumes/tempdisk/compile/x265Neon/source

cmake -DCMAKE_INSTALL_PREFIX:PATH=${SOURCE} -DEXTRA_LIB="x265_main10.a;x265_main12.a" -DEXTRA_LINK_FLAGS=-L. -DLINKED_10BIT=ON -DLINKED_12BIT=ON -DENABLE_SHARED=OFF -DENABLE_CLI=OFF .

make clean

make

mv libx265.a libx265_main.a

libtool -static -o libx265.a libx265_main.a libx265_main10.a libx265_main12.a 2>/dev/null

make install

sleep 1

echo '♻️ ' Start compiling VPX

#
# VPX
#

cd ${COMPILED}

cd libvpx

./configure --prefix=${SOURCE} --enable-vp8 --enable-postproc --enable-vp9-postproc --enable-vp9-highbitdepth --disable-examples --disable-docs --enable-multi-res-encoding --disable-unit-tests --enable-pic --disable-shared

make -j 10

make install

echo '♻️ ' Start compiling EXPAT

#
# EXPAT
#

cd ${COMPILED}

cd expat-2.4.8

./configure --prefix=${SOURCE} --disable-shared --enable-static

make -j 10

make install

echo '♻️ ' Start compiling Gettext

#
# Gettext
#

cd ${COMPILED}

cd gettext-0.21.1

./configure --prefix=${SOURCE} --disable-dependency-tracking --disable-silent-rules --disable-debug --with-included-gettext --with-included-glib \
--with-included-libcroco --with-included-libunistring --with-included-libxml --with-emacs --disable-java --disable-native-java --disable-csharp \
--disable-shared --enable-static --without-git --without-cvs --disable-docs --disable-examples

make -j 10

make install

echo '♻️ ' Start compiling LIBPNG

#
# LIBPNG
#

cd ${COMPILED}

cd libpng-1.6.37

./configure --prefix=${SOURCE} --disable-dependency-tracking --disable-silent-rules --enable-static --disable-shared

make -j 10

make install

echo '♻️ ' Start compiling ENCA

#
# ENCA
#

cd ${COMPILED}

cd enca-1.19

./configure --prefix=${SOURCE} --disable-shared --enable-static

make -j 10

make install

echo '♻️ ' Start compiling FRIBIDI

#
# FRIBIDI
#

cd ${COMPILED}

cd fribidi-1.0.12

./configure --prefix=${SOURCE} --disable-shared --enable-static --disable-silent-rules --disable-debug --disable-dependency-tracking

make -j 10

make install

echo '♻️ ' Start compiling FREETYPE

#
# FREETYPE
#

cd ${COMPILED}

cd freetype-2.12.1

# pip3 install docwriter

./configure --prefix=${SOURCE} --disable-shared --enable-static --enable-freetype-config

make -j 10

make install

echo '♻️ ' Start compiling FONTCONFIG

#
# FONTCONFIG
#

cd ${COMPILED}

cd fontconfig-2.14.2

./configure --prefix=${SOURCE} --enable-iconv --disable-libxml2 --disable-dependency-tracking --disable-silent-rules --disable-shared --enable-static

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling harfbuzz

#
# HARFBUZZ
#

cd ${COMPILED}

cd harfbuzz-6.0.0

./configure --prefix=${SOURCE} --disable-shared --enable-static

make -j 10

make install

echo '♻️ ' Start compiling LIBASS

sleep 1

echo '♻️ ' Start compiling SDL

#
# compile SDL
#

cd ${COMPILED}

cd SDL2-2.26.2

./autogen.sh

./configure --prefix=${SOURCE} --disable-shared --enable-static --without-x --enable-hidapi

make -j 14

make install

#
# LIBASS
#

cd ${COMPILED}

cd libass-0.17.0

./configure --prefix=${SOURCE} --disable-fontconfig --disable-shared --enable-static

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling OPUS

#
# OPUS
#

cd ${COMPILED}

cd opus-1.3.1

./configure --prefix=${SOURCE} --disable-shared --enable-static

make -j 10

make install

sleep 1

#
# LIBOGG
#

cd ${COMPILED}

cd libogg-1.3.5

./configure --prefix=${SOURCE} --disable-shared --enable-static --disable-dependency-tracking

make -j 10

make install

sleep 1

#
# LIBVORBIS
#

cd ${COMPILED}

cd libvorbis-1.3.7

./configure --prefix=${SOURCE} --with-ogg-libraries=${SOURCE}/lib --with-ogg-includes=${SOURCE}/include/ --enable-static --disable-shared

make -j 10

make install

sleep 1

#
# THEORA
#

cd ${COMPILED}

cd libtheora-1.1.1

./configure --prefix=${SOURCE} --disable-asm --with-ogg-libraries=${SOURCE}/lib --with-ogg-includes=${SOURCE}/include/ --with-vorbis-libraries=${SOURCE}/lib --with-vorbis-includes=${SOURCE}/include/ --enable-static --disable-shared

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling Vid-stab

#
# Vidstab
#

cd ${COMPILED}

cd vidstab-master

cmake -DCMAKE_INSTALL_PREFIX:PATH=${SOURCE} -DLIBTYPE=STATIC -DBUILD_SHARED_LIBS=OFF -DUSE_OMP=OFF -DENABLE_SHARED=off .

make -j 10

make install

sleep 1

#
# SNAPPY
#

cd ${COMPILED}

cd snappy

cmake -DCMAKE_INSTALL_PREFIX:PATH=${SOURCE} -DENABLE_SHARED=OFF -DENABLE_CLI=OFF

make -j 10

make install

echo '♻️ ' Start compiling OpenJPEG

sleep 1

#
# OpenJPEG
#

cd ${COMPILED}

cd openjpeg-2.5.0

cmake -DCMAKE_INSTALL_PREFIX:PATH=${SOURCE} -DENABLE_C_DEPS=ON -DLIBTYPE=STATIC -DENABLE_SHARED=OFF -DENABLE_STATIC=ON .

make -j 10

make install

rm ${SOURCE}/lib/libopenjp2.2.5.0.dy*
rm ${SOURCE}/lib/libopenjp2.dy*
rm ${SOURCE}/lib/libopenjp2.7.dy*

Sleep 1

echo '♻️ ' Start compiling AOM

#
# AOM
#

cd ${COMPILED}

cd aom

mkdir aom_build

cd aom_build

cmake ${COMPILED}/aom -DENABLE_TESTS=0 -DCMAKE_INSTALL_PREFIX:PATH=${SOURCE} -DLIBTYPE=STATIC -DAOM_TARGET_CPU=ARM64 -DCONFIG_RUNTIME_CPU_DETECT=0

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling WEBP

#
# WEBP
#

cd ${COMPILED}

cd libwebp-1.1.0

cmake -DCMAKE_INSTALL_PREFIX:PATH=${SOURCE} -DENABLE_C_DEPS=ON -DLIBTYPE=STATIC -DENABLE_SHARED=OFF -DENABLE_STATIC=ON .

make -j 10

make install

sleep 1

echo '♻️ ' compile zimg

cd ${COMPILED}

cd zimg-release-3.0.2

./autogen.sh

./Configure --prefix=${SOURCE} --disable-shared --enable-static

make -j 10

make install

echo '♻️ ' Start compiling SVT-AV1

#
# SVT-AV1
#

cd ${COMPILED}

cd SVT-AV1-master

cmake -DCMAKE_INSTALL_PREFIX:PATH=${SOURCE} -DCMAKE_BUILD_TYPE=Release -DBUILD_DEC=OFF -DBUILD_SHARED_LIBS=OFF -DLIBTYPE=STATIC -DENABLE_SHARED=OFF -DENABLE_STATIC=ON .

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling KVAZAAR

#
# KVAZAAR
#

cd ${COMPILED}

cd kvazaar-2.2.0

./configure --prefix=${SOURCE} --disable-shared --enable-static

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling libudfread

#
# libudfread
#

cd ${COMPILED}

cd libudfread

./bootstrap

./configure --prefix=${SOURCE} --disable-shared --enable-static

make -j 10

make install

sleep 1

echo '♻️ ' Start compiling libbluray

#
# compile libbluray
#

cd ${COMPILED}

cd libbluray

./bootstrap

./configure --prefix=${SOURCE} --disable-shared --enable-static --disable-dependency-tracking --disable-silent-rules --without-libxml2 --without-freetype --disable-doxygen-doc --disable-bdjava-jar

make -j 14

make install

echo '♻️ ' Start compiling FFMPEG

cd ${COMPILED}

cd ffmpeg

export LDFLAGS="-L${SOURCE}/lib"

export CFLAGS="-I${SOURCE}/include"

export LDFLAGS="$LDFLAGS -framework VideoToolbox"

./configure --prefix=${SOURCE} --extra-cflags="-fno-stack-check" --arch=arm64 --cc=/usr/bin/clang --enable-gpl --enable-libbluray --enable-libopenjpeg --enable-libopus --enable-libmp3lame --enable-libx264 --enable-libx265 --enable-libvpx --enable-libwebp --enable-libass --enable-libfreetype --enable-fontconfig --enable-libtheora --enable-libvorbis --enable-libsnappy --enable-libaom --enable-libvidstab --enable-libzimg --enable-libsvtav1 --enable-libkvazaar --enable-libharfbuzz --pkg-config-flags=--static --enable-ffplay --enable-postproc --enable-neon --enable-runtime-cpudetect --disable-indev=qtkit --disable-indev=x11grab_xcb

make -j 10

make install"#;
    
    std::fs::write(compiling_path, compiling_text)?;
    Ok(())
}

use rocket::serde::json::Json;
use rocket::{get, post};
use crate::web::ApiResponse;
use crate::auth::AuthGuard;
use sha2::{Sha256, Digest};

fn verify_sha256(file_path: &str, expected_hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let file_contents = std::fs::read(file_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&file_contents);
    let computed_hash = format!("{:x}", hasher.finalize());
    
    Ok(computed_hash == expected_hash)
}

fn get_tool_sha256(tool: &str, platform: &str) -> Option<&'static str> {
    match tool {
        "ffmpeg" => get_ffmpeg_sha256(platform),
        "ffprobe" => get_ffprobe_sha256(platform),
        "whisper-cli" => get_whisper_cli_sha256(platform),
        _ => None,
    }
}

#[derive(serde::Deserialize)]
pub struct DownloadRequest {
    tool: String,
}

#[derive(serde::Deserialize)]
pub struct UseDownloadedRequest {
    tool_name: String,
}

#[get("/api/tools/list")]
pub fn web_list_tools(_auth: AuthGuard) -> Json<ApiResponse<Vec<ToolInfo>>> {
    let tools = list_tools();
    Json(ApiResponse::success(tools))
}

#[post("/api/tools/download", data = "<request>")]
pub fn web_download_tool(_auth: AuthGuard, request: Json<DownloadRequest>) -> Json<ApiResponse<String>> {
    match download_tool(&request.tool) {
        Ok(path) => Json(ApiResponse::success(path)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

#[post("/api/tools/use-downloaded", data = "<request>")]
pub fn web_use_downloaded_tool(_auth: AuthGuard, request: Json<UseDownloadedRequest>) -> Json<ApiResponse<String>> {
    let downloaded_path = get_downloaded_path(&request.tool_name);
    
    if std::path::Path::new(&downloaded_path).exists() {
        match crate::config::load_config() {
            Ok(mut cfg) => {
                let config_field = match request.tool_name.as_str() {
                    "ffmpeg" => "ffmpeg_path",
                    "ffprobe" => "ffprobe_path", 
                    "whisper-cli" => "whispercli_path",
                    _ => return Json(ApiResponse::error(format!("Unknown tool: {}", request.tool_name))),
                };
                
                if let Err(e) = crate::config::set_config_field(&mut cfg, config_field, &downloaded_path) {
                    return Json(ApiResponse::error(format!("Error setting config field: {}", e)));
                }
                
                if let Err(e) = crate::config::store_config(&cfg) {
                    return Json(ApiResponse::error(format!("Error saving config: {}", e)));
                }
                
                Json(ApiResponse::success(format!("Set {} to use downloaded tool at {}", config_field, downloaded_path)))
            }
            Err(e) => Json(ApiResponse::error(format!("Error loading config: {}", e))),
        }
    } else {
        Json(ApiResponse::error(format!("Tool '{}' is not downloaded", request.tool_name)))
    }
}

#[cfg(target_os = "macos")]
pub fn handle_macos_quarantine(executable_path: &str, platform: &str) -> Result<(), Box<dyn std::error::Error>> {
    check_macos_version()?;
    remove_quarantine(executable_path)?;
    handle_arm_mac_signing(executable_path, platform)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn check_macos_version() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()?;

    if !output.status.success() {
        return Err("Unable to get macOS version".into());
    }

    let version_string = String::from_utf8(output.stdout)?;
    let version_parts: Vec<&str> = version_string.trim().split('.').collect();

    if version_parts.len() < 2 {
        return Err("Unable to parse macOS version".into());
    }

    let major: i32 = version_parts[0].parse()?;
    let minor: i32 = version_parts[1].parse()?;

    if major > 10 || (major == 10 && minor >= 15) {
        Ok(())
    } else {
        Err("macOS version too old for quarantine handling".into())
    }
}

#[cfg(target_os = "macos")]
fn remove_quarantine(executable_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Removing quarantine from {}", executable_path);

    let output = std::process::Command::new("xattr")
        .args(["-dr", "com.apple.quarantine", executable_path])
        .output()?;

    if output.status.success() {
        println!("Successfully removed quarantine");
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("xattr command failed with exit code {:?}: {}", output.status.code(), error_msg);
        Err(format!("Failed to remove quarantine: {}", error_msg).into())
    }
}

#[cfg(target_os = "macos")]
fn handle_arm_mac_signing(executable_path: &str, platform: &str) -> Result<(), Box<dyn std::error::Error>> {
    if platform == "macos-arm" {
        println!("Handling ARM Mac code signing for {}", executable_path);
        clear_extended_attributes(executable_path)?;
        codesign_executable(executable_path)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn clear_extended_attributes(executable_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Clearing extended attributes");

    let output = std::process::Command::new("xattr")
        .args(["-cr", executable_path])
        .output()?;

    if output.status.success() {
        println!("Successfully cleared extended attributes");
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("xattr -cr command failed with exit code {:?}: {}", output.status.code(), error_msg);
        Err(format!("Failed to clear extended attributes: {}", error_msg).into())
    }
}

#[cfg(target_os = "macos")]
fn codesign_executable(executable_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Code signing executable");

    let output = std::process::Command::new("codesign")
        .args(["-s", "-", executable_path])
        .output()?;

    if output.status.success() {
        println!("Successfully code signed executable");
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        eprintln!("codesign command failed with exit code {:?}: {}", output.status.code(), error_msg);
        Err(format!("Failed to code sign executable: {}", error_msg).into())
    }
}