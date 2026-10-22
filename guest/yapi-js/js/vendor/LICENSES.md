# Vendored packages

The modules in this directory are esbuild bundles of npm packages that pi
extensions import. They are generated, not edited; the bundles share chunks so
each package is included once.

| Package | Version | License | Copyright |
|---|---|---|---|
| `@earendil-works/pi-ai` (the parts `xtask/vendor-pi/pi-ai/entry.mjs` names, with yapi's replacements for the modules that call provider SDKs) | 1.0.0 | MIT | Copyright (c) 2025 Mario Zechner |
| `@earendil-works/pi-tui` | 1.0.0 | MIT | Copyright (c) 2025 Mario Zechner |
| `@earendil-works/pi-agent-core` | 1.0.0 | MIT | Copyright (c) 2025 Mario Zechner |
| `@earendil-works/pi-coding-agent` (the modules its facade serves) | 1.0.0 | MIT | Copyright (c) 2025 Mario Zechner |
| `typebox` | 1.3.27 | MIT | Copyright (c) 2017-2026 Haydn Paterson |
| `marked` (bundled by pi-tui) | 18.0.11 | MIT | Copyright (c) 2018+, MarkedJS; Copyright (c) 2011-2018, Christopher Jeffrey |
| `get-east-asian-width` (bundled by pi-tui) | 1.6.0 | MIT | Copyright (c) Sindre Sorhus |
| `balanced-match` (bundled by pi-coding-agent) | 4.0.4 | MIT | Copyright Julian Gruber |
| `brace-expansion` (bundled by pi-coding-agent) | 5.0.9 | MIT | Copyright Julian Gruber |
| `cross-spawn` (bundled by pi-coding-agent) | 7.0.6 | MIT | Copyright (c) 2018 Made With MOXY Lda |
| `diff` (bundled by pi-coding-agent) | 8.0.4 | BSD-3-Clause | Copyright (c) 2009-2015, Kevin Decker |
| `graceful-fs` (bundled by pi-coding-agent) | 4.2.11 | ISC | Copyright (c) 2011-2022 Isaac Z. Schlueter, Ben Noordhuis, and Contributors |
| `ignore` (bundled by pi-coding-agent) | 7.0.8 | MIT | Copyright (c) 2013 Kael Zhang, contributors |
| `isexe` (bundled by pi-coding-agent) | 2.0.0 | ISC | Copyright (c) Isaac Z. Schlueter and Contributors |
| `minimatch` (bundled by pi-coding-agent) | 10.2.6 | BlueOak-1.0.0 | Isaac Z. Schlueter and Contributors |
| `partial-json` (bundled by pi-ai) | 0.1.7 | MIT | Copyright (c) 2023 Promplate Dev Team |
| `path-key` (bundled by pi-coding-agent) | 3.1.1 | MIT | Copyright (c) Sindre Sorhus |
| `proper-lockfile` (bundled by pi-coding-agent) | 4.1.2 | MIT | Copyright (c) 2018 Made With MOXY Lda |
| `retry` (bundled by pi-coding-agent) | 0.13.1 | MIT | Copyright (c) 2011 Tim Koschützki, Felix Geisendörfer |
| `shebang-command` (bundled by pi-coding-agent) | 2.0.0 | MIT | Copyright (c) Kevin Mårtensson |
| `shebang-regex` (bundled by pi-coding-agent) | 3.0.0 | MIT | Copyright (c) Sindre Sorhus |
| `signal-exit` (bundled by pi-coding-agent) | 3.0.7 | ISC | Copyright (c) 2015, Contributors |
| `which` (bundled by pi-coding-agent) | 2.0.2 | ISC | Copyright (c) Isaac Z. Schlueter and Contributors |
| `@earendil-works/pi-codemode` (`codemode-prelude.js`, the sandbox prelude, unchanged) | 1.0.0 | MIT | Copyright (c) 2025 Mario Zechner |

The facades in `../pi` port small parts of pi-ai and pi-coding-agent 1.0.0
(MIT, Copyright (c) 2025 Mario Zechner). The pi-coding-agent bundle replaces
pi's configuration, theme, frontmatter and image modules with `yapi:pi/host`.

## MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## ISC License

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR
IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

## BSD 3-Clause License

Copyright (c) 2009-2015, Kevin Decker <kpdecker@gmail.com>
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## Blue Oak Model License

Version 1.0.0

### Purpose

This license gives everyone as much permission to work with
this software as possible, while protecting contributors
from liability.

### Acceptance

In order to receive this license, you must agree to its
rules. The rules of this license are both obligations
under that agreement and conditions to your license.
You must not do anything with this software that triggers
a rule that you cannot or will not follow.

### Copyright

Each contributor licenses you to do everything with this
software that would otherwise infringe that contributor's
copyright in it.

### Notices

You must ensure that everyone who gets a copy of
any part of this software from you, with or without
changes, also gets the text of this license or a link to
<https://blueoakcouncil.org/license/1.0.0>.

### Excuse

If anyone notifies you in writing that you have not
complied with [Notices](#notices), you can keep your
license by taking all practical steps to comply within 30
days after the notice. If you do not do so, your license
ends immediately.

### Patent

Each contributor licenses you to do everything with this
software that would otherwise infringe any patent claims
they can license or become able to license.

### Reliability

No contributor can revoke this license.

### No Liability

**_As far as the law allows, this software comes as is,
without any warranty or condition, and no contributor
will be liable to anyone for any damages related to this
software or this license, under any kind of legal claim._**
