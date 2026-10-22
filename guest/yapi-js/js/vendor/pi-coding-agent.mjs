const require = globalThis.__yapi_require_for('yapi:vendor');
import {
  typebox_exports
} from "./chunk-OX5HQF2L.mjs";
import "./chunk-A5T7ZXDE.mjs";
import {
  Box,
  Container,
  Editor,
  Image,
  Input,
  KeybindingsManager,
  Loader,
  Markdown,
  MouseRegion,
  SelectList,
  SettingsList,
  Spacer,
  TUI_KEYBINDINGS,
  Text,
  TruncatedText,
  fuzzyFilter,
  fuzzyMatch,
  getCapabilities,
  getImageDimensions,
  getKeybindings,
  hyperlink,
  imageFallback,
  sliceByColumn,
  truncateToWidth,
  visibleWidth,
  wrapTextWithAnsi
} from "./chunk-EGLY2VUN.mjs";
import {
  __commonJS,
  __reExport,
  __require,
  __toESM
} from "./chunk-G7YUWZG4.mjs";

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/isexe/windows.js
var require_windows = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/isexe/windows.js"(exports, module) {
    module.exports = isexe;
    isexe.sync = sync;
    var fs = __require("fs");
    function checkPathExt(path4, options) {
      var pathext = options.pathExt !== void 0 ? options.pathExt : process.env.PATHEXT;
      if (!pathext) {
        return true;
      }
      pathext = pathext.split(";");
      if (pathext.indexOf("") !== -1) {
        return true;
      }
      for (var i = 0; i < pathext.length; i++) {
        var p = pathext[i].toLowerCase();
        if (p && path4.substr(-p.length).toLowerCase() === p) {
          return true;
        }
      }
      return false;
    }
    function checkStat(stat2, path4, options) {
      if (!stat2.isSymbolicLink() && !stat2.isFile()) {
        return false;
      }
      return checkPathExt(path4, options);
    }
    function isexe(path4, options, cb) {
      fs.stat(path4, function(er, stat2) {
        cb(er, er ? false : checkStat(stat2, path4, options));
      });
    }
    function sync(path4, options) {
      return checkStat(fs.statSync(path4), path4, options);
    }
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/isexe/mode.js
var require_mode = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/isexe/mode.js"(exports, module) {
    module.exports = isexe;
    isexe.sync = sync;
    var fs = __require("fs");
    function isexe(path4, options, cb) {
      fs.stat(path4, function(er, stat2) {
        cb(er, er ? false : checkStat(stat2, options));
      });
    }
    function sync(path4, options) {
      return checkStat(fs.statSync(path4), options);
    }
    function checkStat(stat2, options) {
      return stat2.isFile() && checkMode(stat2, options);
    }
    function checkMode(stat2, options) {
      var mod = stat2.mode;
      var uid = stat2.uid;
      var gid = stat2.gid;
      var myUid = options.uid !== void 0 ? options.uid : process.getuid && process.getuid();
      var myGid = options.gid !== void 0 ? options.gid : process.getgid && process.getgid();
      var u = parseInt("100", 8);
      var g = parseInt("010", 8);
      var o = parseInt("001", 8);
      var ug = u | g;
      var ret = mod & o || mod & g && gid === myGid || mod & u && uid === myUid || mod & ug && myUid === 0;
      return ret;
    }
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/isexe/index.js
var require_isexe = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/isexe/index.js"(exports, module) {
    var fs = __require("fs");
    var core;
    if (process.platform === "win32" || global.TESTING_WINDOWS) {
      core = require_windows();
    } else {
      core = require_mode();
    }
    module.exports = isexe;
    isexe.sync = sync;
    function isexe(path4, options, cb) {
      if (typeof options === "function") {
        cb = options;
        options = {};
      }
      if (!cb) {
        if (typeof Promise !== "function") {
          throw new TypeError("callback not provided");
        }
        return new Promise(function(resolve5, reject) {
          isexe(path4, options || {}, function(er, is) {
            if (er) {
              reject(er);
            } else {
              resolve5(is);
            }
          });
        });
      }
      core(path4, options || {}, function(er, is) {
        if (er) {
          if (er.code === "EACCES" || options && options.ignoreErrors) {
            er = null;
            is = false;
          }
        }
        cb(er, is);
      });
    }
    function sync(path4, options) {
      try {
        return core.sync(path4, options || {});
      } catch (er) {
        if (options && options.ignoreErrors || er.code === "EACCES") {
          return false;
        } else {
          throw er;
        }
      }
    }
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/which/which.js
var require_which = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/which/which.js"(exports, module) {
    var isWindows = process.platform === "win32" || process.env.OSTYPE === "cygwin" || process.env.OSTYPE === "msys";
    var path4 = __require("path");
    var COLON = isWindows ? ";" : ":";
    var isexe = require_isexe();
    var getNotFoundError2 = (cmd) => Object.assign(new Error(`not found: ${cmd}`), { code: "ENOENT" });
    var getPathInfo = (cmd, opt) => {
      const colon = opt.colon || COLON;
      const pathEnv = cmd.match(/\//) || isWindows && cmd.match(/\\/) ? [""] : [
        // windows always checks the cwd first
        ...isWindows ? [process.cwd()] : [],
        ...(opt.path || process.env.PATH || /* istanbul ignore next: very unusual */
        "").split(colon)
      ];
      const pathExtExe = isWindows ? opt.pathExt || process.env.PATHEXT || ".EXE;.CMD;.BAT;.COM" : "";
      const pathExt = isWindows ? pathExtExe.split(colon) : [""];
      if (isWindows) {
        if (cmd.indexOf(".") !== -1 && pathExt[0] !== "")
          pathExt.unshift("");
      }
      return {
        pathEnv,
        pathExt,
        pathExtExe
      };
    };
    var which = (cmd, opt, cb) => {
      if (typeof opt === "function") {
        cb = opt;
        opt = {};
      }
      if (!opt)
        opt = {};
      const { pathEnv, pathExt, pathExtExe } = getPathInfo(cmd, opt);
      const found = [];
      const step = (i) => new Promise((resolve5, reject) => {
        if (i === pathEnv.length)
          return opt.all && found.length ? resolve5(found) : reject(getNotFoundError2(cmd));
        const ppRaw = pathEnv[i];
        const pathPart = /^".*"$/.test(ppRaw) ? ppRaw.slice(1, -1) : ppRaw;
        const pCmd = path4.join(pathPart, cmd);
        const p = !pathPart && /^\.[\\\/]/.test(cmd) ? cmd.slice(0, 2) + pCmd : pCmd;
        resolve5(subStep(p, i, 0));
      });
      const subStep = (p, i, ii) => new Promise((resolve5, reject) => {
        if (ii === pathExt.length)
          return resolve5(step(i + 1));
        const ext2 = pathExt[ii];
        isexe(p + ext2, { pathExt: pathExtExe }, (er, is) => {
          if (!er && is) {
            if (opt.all)
              found.push(p + ext2);
            else
              return resolve5(p + ext2);
          }
          return resolve5(subStep(p, i, ii + 1));
        });
      });
      return cb ? step(0).then((res) => cb(null, res), cb) : step(0);
    };
    var whichSync = (cmd, opt) => {
      opt = opt || {};
      const { pathEnv, pathExt, pathExtExe } = getPathInfo(cmd, opt);
      const found = [];
      for (let i = 0; i < pathEnv.length; i++) {
        const ppRaw = pathEnv[i];
        const pathPart = /^".*"$/.test(ppRaw) ? ppRaw.slice(1, -1) : ppRaw;
        const pCmd = path4.join(pathPart, cmd);
        const p = !pathPart && /^\.[\\\/]/.test(cmd) ? cmd.slice(0, 2) + pCmd : pCmd;
        for (let j = 0; j < pathExt.length; j++) {
          const cur = p + pathExt[j];
          try {
            const is = isexe.sync(cur, { pathExt: pathExtExe });
            if (is) {
              if (opt.all)
                found.push(cur);
              else
                return cur;
            }
          } catch (ex) {
          }
        }
      }
      if (opt.all && found.length)
        return found;
      if (opt.nothrow)
        return null;
      throw getNotFoundError2(cmd);
    };
    module.exports = which;
    which.sync = whichSync;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/path-key/index.js
var require_path_key = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/path-key/index.js"(exports, module) {
    "use strict";
    var pathKey = (options = {}) => {
      const environment = options.env || process.env;
      const platform2 = options.platform || process.platform;
      if (platform2 !== "win32") {
        return "PATH";
      }
      return Object.keys(environment).reverse().find((key) => key.toUpperCase() === "PATH") || "Path";
    };
    module.exports = pathKey;
    module.exports.default = pathKey;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/util/resolveCommand.js
var require_resolveCommand = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/util/resolveCommand.js"(exports, module) {
    "use strict";
    var path4 = __require("path");
    var which = require_which();
    var getPathKey = require_path_key();
    function resolveCommandAttempt(parsed, withoutPathExt) {
      const env = parsed.options.env || process.env;
      const cwd = process.cwd();
      const hasCustomCwd = parsed.options.cwd != null;
      const shouldSwitchCwd = hasCustomCwd && process.chdir !== void 0 && !process.chdir.disabled;
      if (shouldSwitchCwd) {
        try {
          process.chdir(parsed.options.cwd);
        } catch (err) {
        }
      }
      let resolved;
      try {
        resolved = which.sync(parsed.command, {
          path: env[getPathKey({ env })],
          pathExt: withoutPathExt ? path4.delimiter : void 0
        });
      } catch (e) {
      } finally {
        if (shouldSwitchCwd) {
          process.chdir(cwd);
        }
      }
      if (resolved) {
        resolved = path4.resolve(hasCustomCwd ? parsed.options.cwd : "", resolved);
      }
      return resolved;
    }
    function resolveCommand(parsed) {
      return resolveCommandAttempt(parsed) || resolveCommandAttempt(parsed, true);
    }
    module.exports = resolveCommand;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/util/escape.js
var require_escape = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/util/escape.js"(exports, module) {
    "use strict";
    var metaCharsRegExp = /([()\][%!^"`<>&|;, *?])/g;
    function escapeCommand(arg) {
      arg = arg.replace(metaCharsRegExp, "^$1");
      return arg;
    }
    function escapeArgument(arg, doubleEscapeMetaChars) {
      arg = `${arg}`;
      arg = arg.replace(/(?=(\\+?)?)\1"/g, '$1$1\\"');
      arg = arg.replace(/(?=(\\+?)?)\1$/, "$1$1");
      arg = `"${arg}"`;
      arg = arg.replace(metaCharsRegExp, "^$1");
      if (doubleEscapeMetaChars) {
        arg = arg.replace(metaCharsRegExp, "^$1");
      }
      return arg;
    }
    module.exports.command = escapeCommand;
    module.exports.argument = escapeArgument;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/shebang-regex/index.js
var require_shebang_regex = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/shebang-regex/index.js"(exports, module) {
    "use strict";
    module.exports = /^#!(.*)/;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/shebang-command/index.js
var require_shebang_command = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/shebang-command/index.js"(exports, module) {
    "use strict";
    var shebangRegex = require_shebang_regex();
    module.exports = (string = "") => {
      const match2 = string.match(shebangRegex);
      if (!match2) {
        return null;
      }
      const [path4, argument] = match2[0].replace(/#! ?/, "").split(" ");
      const binary = path4.split("/").pop();
      if (binary === "env") {
        return argument;
      }
      return argument ? `${binary} ${argument}` : binary;
    };
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/util/readShebang.js
var require_readShebang = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/util/readShebang.js"(exports, module) {
    "use strict";
    var fs = __require("fs");
    var shebangCommand = require_shebang_command();
    function readShebang(command) {
      const size = 150;
      const buffer = Buffer.alloc(size);
      let fd;
      try {
        fd = fs.openSync(command, "r");
        fs.readSync(fd, buffer, 0, size, 0);
        fs.closeSync(fd);
      } catch (e) {
      }
      return shebangCommand(buffer.toString());
    }
    module.exports = readShebang;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/parse.js
var require_parse = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/parse.js"(exports, module) {
    "use strict";
    var path4 = __require("path");
    var resolveCommand = require_resolveCommand();
    var escape2 = require_escape();
    var readShebang = require_readShebang();
    var isWin = process.platform === "win32";
    var isExecutableRegExp = /\.(?:com|exe)$/i;
    var isCmdShimRegExp = /node_modules[\\/].bin[\\/][^\\/]+\.cmd$/i;
    function detectShebang(parsed) {
      parsed.file = resolveCommand(parsed);
      const shebang = parsed.file && readShebang(parsed.file);
      if (shebang) {
        parsed.args.unshift(parsed.file);
        parsed.command = shebang;
        return resolveCommand(parsed);
      }
      return parsed.file;
    }
    function parseNonShell(parsed) {
      if (!isWin) {
        return parsed;
      }
      const commandFile = detectShebang(parsed);
      const needsShell = !isExecutableRegExp.test(commandFile);
      if (parsed.options.forceShell || needsShell) {
        const needsDoubleEscapeMetaChars = isCmdShimRegExp.test(commandFile);
        parsed.command = path4.normalize(parsed.command);
        parsed.command = escape2.command(parsed.command);
        parsed.args = parsed.args.map((arg) => escape2.argument(arg, needsDoubleEscapeMetaChars));
        const shellCommand = [parsed.command].concat(parsed.args).join(" ");
        parsed.args = ["/d", "/s", "/c", `"${shellCommand}"`];
        parsed.command = process.env.comspec || "cmd.exe";
        parsed.options.windowsVerbatimArguments = true;
      }
      return parsed;
    }
    function parse(command, args, options) {
      if (args && !Array.isArray(args)) {
        options = args;
        args = null;
      }
      args = args ? args.slice(0) : [];
      options = Object.assign({}, options);
      const parsed = {
        command,
        args,
        options,
        file: void 0,
        original: {
          command,
          args
        }
      };
      return options.shell ? parsed : parseNonShell(parsed);
    }
    module.exports = parse;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/enoent.js
var require_enoent = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/lib/enoent.js"(exports, module) {
    "use strict";
    var isWin = process.platform === "win32";
    function notFoundError(original, syscall) {
      return Object.assign(new Error(`${syscall} ${original.command} ENOENT`), {
        code: "ENOENT",
        errno: "ENOENT",
        syscall: `${syscall} ${original.command}`,
        path: original.command,
        spawnargs: original.args
      });
    }
    function hookChildProcess(cp, parsed) {
      if (!isWin) {
        return;
      }
      const originalEmit = cp.emit;
      cp.emit = function(name, arg1) {
        if (name === "exit") {
          const err = verifyENOENT(arg1, parsed);
          if (err) {
            return originalEmit.call(cp, "error", err);
          }
        }
        return originalEmit.apply(cp, arguments);
      };
    }
    function verifyENOENT(status, parsed) {
      if (isWin && status === 1 && !parsed.file) {
        return notFoundError(parsed.original, "spawn");
      }
      return null;
    }
    function verifyENOENTSync(status, parsed) {
      if (isWin && status === 1 && !parsed.file) {
        return notFoundError(parsed.original, "spawnSync");
      }
      return null;
    }
    module.exports = {
      hookChildProcess,
      verifyENOENT,
      verifyENOENTSync,
      notFoundError
    };
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/index.js
var require_cross_spawn = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/cross-spawn/index.js"(exports, module) {
    "use strict";
    var cp = __require("child_process");
    var parse = require_parse();
    var enoent = require_enoent();
    function spawn7(command, args, options) {
      const parsed = parse(command, args, options);
      const spawned = cp.spawn(parsed.command, parsed.args, parsed.options);
      enoent.hookChildProcess(spawned, parsed);
      return spawned;
    }
    function spawnSync5(command, args, options) {
      const parsed = parse(command, args, options);
      const result = cp.spawnSync(parsed.command, parsed.args, parsed.options);
      result.error = result.error || enoent.verifyENOENTSync(result.status, parsed);
      return result;
    }
    module.exports = spawn7;
    module.exports.spawn = spawn7;
    module.exports.sync = spawnSync5;
    module.exports._parse = parse;
    module.exports._enoent = enoent;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/ignore/index.js
var require_ignore = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/ignore/index.js"(exports, module) {
    function makeArray(subject) {
      return Array.isArray(subject) ? subject : [subject];
    }
    var UNDEFINED = void 0;
    var EMPTY = "";
    var SPACE = " ";
    var ESCAPE = "\\";
    var REGEX_LITERAL_SPECIAL = /[.*+?()[\]{}^$|\\/]/;
    var REGEX_TEST_BLANK_LINE = /^ +$/;
    var REGEX_INVALID_TRAILING_BACKSLASH = /(?:[^\\]|^)\\$/;
    var REGEX_REPLACE_LEADING_EXCAPED_EXCLAMATION = /^\\!/;
    var REGEX_REPLACE_LEADING_EXCAPED_HASH = /^\\#/;
    var REGEX_SPLITALL_CRLF = /\r?\n/g;
    var DOUBLE_SLASH = "//";
    var SLASH_CODE = 47;
    var DOT_CODE = 46;
    var SLASH = "/";
    var TMP_KEY_IGNORE = "node-ignore";
    if (typeof Symbol !== "undefined") {
      TMP_KEY_IGNORE = Symbol.for("node-ignore");
    }
    var KEY_IGNORE = TMP_KEY_IGNORE;
    var define = (object, key, value) => {
      Object.defineProperty(object, key, { value });
      return value;
    };
    var RETURN_FALSE = () => false;
    var cleanRangeBackSlash = (slashes) => {
      const { length } = slashes;
      return slashes.slice(0, length - length % 2);
    };
    var POSIX_CLASSES = {
      alnum: "0-9A-Za-z",
      alpha: "A-Za-z",
      blank: " \\t",
      cntrl: "\\x00-\\x1f\\x7f",
      digit: "0-9",
      graph: "!-.0-~",
      lower: "a-z",
      print: " -.0-~",
      punct: "!-.:-@\\[-`{-~",
      // git's `sane-ctype.h` classifies \v and \f as control, not space,
      //   unlike C's `isspace`
      space: " \\t\\n\\r",
      upper: "A-Z",
      xdigit: "0-9A-Fa-f"
    };
    var CLASS_MEMBERS_TO_ESCAPE = "\\]^-[";
    var escapeMember = (char) => CLASS_MEMBERS_TO_ESCAPE.indexOf(char) < 0 ? char : ESCAPE + char;
    var NON_SLASH = "(?!\\/)";
    var classSource = (negated, body) => {
      if (negated) {
        return `[^\\/${body}]`;
      }
      const source = `[${body}]`;
      return new RegExp(source).test("/") ? NON_SLASH + source : source;
    };
    var scanBracket = (pattern, start) => {
      const { length } = pattern;
      let index = start + 1;
      let negated = EMPTY;
      const lead = pattern[index];
      if (lead === "!" || lead === "^") {
        negated = "^";
        index++;
      }
      let body = EMPTY;
      let prev = EMPTY;
      for (; ; ) {
        const char = pattern[index];
        if (char === UNDEFINED) {
          return null;
        }
        if (char === ESCAPE) {
          const escaped = pattern[index + 1];
          if (escaped === UNDEFINED) {
            return null;
          }
          body += escapeMember(escaped);
          prev = escaped;
          index++;
        } else if (char === "-" && prev && index + 1 < length && pattern[index + 1] !== "]") {
          index++;
          let to = pattern[index];
          if (to === ESCAPE) {
            to = pattern[index += 1];
          }
          if (prev <= to) {
            body += `-${escapeMember(to)}`;
          }
          prev = EMPTY;
        } else if (char === "[" && pattern[index + 1] === ":") {
          const nameStart = index + 2;
          let end = nameStart;
          while (end < length && pattern[end] !== "]") {
            end++;
          }
          if (end === length) {
            return null;
          }
          if (end > nameStart && pattern[end - 1] === ":") {
            const expanded = POSIX_CLASSES[pattern.slice(nameStart, end - 1)];
            if (expanded === UNDEFINED) {
              return null;
            }
            body += expanded;
            prev = EMPTY;
            index = end;
          } else {
            body += escapeMember("[");
            prev = "[";
            index = nameStart - 2;
          }
        } else {
          body += escapeMember(char);
          prev = char;
        }
        index++;
        if (pattern[index] === "]") {
          return {
            end: index,
            source: classSource(negated, body)
          };
        }
      }
    };
    var NEVER_MATCH = "[]";
    var PLACEHOLDER = "\0";
    var REGEX_RESTORE_PLACEHOLDER = new RegExp(
      `${PLACEHOLDER}(\\d+)${PLACEHOLDER}`,
      "g"
    );
    var TRAILING_WILDCARD = "\uE000";
    var extractBrackets = (pattern) => {
      const sources = [];
      const hold = (source) => `${PLACEHOLDER}${sources.push(source) - 1}${PLACEHOLDER}`;
      const { length } = pattern;
      let out = EMPTY;
      let index = 0;
      while (index < length) {
        const char = pattern[index];
        if (char === ESCAPE) {
          const escaped = pattern[index + 1];
          if (escaped === "*" || escaped === "[" || escaped === SPACE || escaped === ESCAPE) {
            out += pattern.slice(index, index + 2);
          } else {
            out += hold(
              REGEX_LITERAL_SPECIAL.test(escaped) ? ESCAPE + escaped : escaped
            );
          }
          index += 2;
        } else if (char === PLACEHOLDER) {
          out += hold(`[${PLACEHOLDER}]`);
          index++;
        } else if (char === "[") {
          const scanned = scanBracket(pattern, index);
          if (scanned === null) {
            out += hold(NEVER_MATCH);
            index = length;
          } else {
            out += hold(scanned.source);
            index = scanned.end + 1;
          }
        } else {
          out += char;
          index++;
        }
      }
      return {
        source: out,
        sources
      };
    };
    var DIRECT = null;
    var REGEX_INNER_SLASH = /\/(?!$)/;
    var REPLACERS = [
      [
        // Remove BOM
        // TODO:
        // Other similar zero-width characters?
        /^\uFEFF/,
        () => EMPTY,
        "\uFEFF"
      ],
      [
        // A trailing line terminator, left on when a whole file's contents are
        //   added as one pattern rather than split into lines. git never sees one
        //   -- it reads a `.gitignore` line by line -- so it is not part of the
        //   pattern and is dropped here, apart from the trailing-space trimming,
        //   which follows git in touching spaces and nothing else.
        /[\r\n]+$/,
        () => EMPTY
      ],
      // > Trailing spaces are ignored unless they are quoted with backslash ("\")
      [
        // Only spaces, never tabs or other whitespace: git trims a trailing run
        //   of `' '` and nothing else (dir.c, `trim_trailing_spaces`, a single
        //   `case ' '`), so a pattern ending in a tab keeps it as a literal.
        // (a\ ) -> (a )
        // (a  ) -> (a)
        // (a ) -> (a)
        // (a \ ) -> (a  )
        /((?:\\\\)*?)(\\? +)$/,
        (_, m1, m2) => m1 + (m2.indexOf("\\") === 0 ? SPACE : EMPTY)
      ],
      // Replace (\ ) with ' '
      // Only a space: an escaped tab or other whitespace is already a literal by
      //   the time it reaches here, and a bare tab must be left as one, not turned
      //   into a space.
      // (\ ) -> ' '
      // (\\ ) -> '\\ '
      // (\\\ ) -> '\\ '
      [
        /(\\+?) /g,
        (_, m1) => {
          const { length } = m1;
          return m1.slice(0, length - length % 2) + SPACE;
        }
      ],
      // Escape metacharacters
      // which is written down by users but means special for regular expressions.
      // > There are 12 characters with special meanings:
      // > - the backslash \,
      // > - the caret ^,
      // > - the dollar sign $,
      // > - the period or dot .,
      // > - the vertical bar or pipe symbol |,
      // > - the question mark ?,
      // > - the asterisk or star *,
      // > - the plus sign +,
      // > - the opening parenthesis (,
      // > - the closing parenthesis ),
      // > - and the opening square bracket [,
      // > - the opening curly brace {,
      // > These special characters are often called "metacharacters".
      [
        /[\\$.|*+(){^]/g,
        (match2) => `\\${match2}`
      ],
      [
        // > a question mark (?) matches a single character
        /(?!\\)\?/g,
        () => "[^/]",
        "?"
      ],
      // leading slash
      [
        // > A leading slash matches the beginning of the pathname.
        // > For example, "/*.c" matches "cat-file.c" but not "mozilla-sha1/sha1.c".
        // A leading slash matches the beginning of the pathname
        /^\//,
        () => "^",
        SLASH
      ],
      // replace special metacharacter slash after the leading slash
      [
        /\//g,
        () => "\\/",
        SLASH
      ],
      [
        // > A leading "**" followed by a slash means match in all directories.
        // > For example, "**/foo" matches file or directory "foo" anywhere,
        // > the same as pattern "foo".
        // > "**/foo/bar" matches file or directory "bar" anywhere that is directly
        // >   under directory "foo".
        // Notice that the '*'s have been replaced as '\\*'
        /^\^*(?:\\\*\\\*\\\/)+/,
        // '**/foo' <-> 'foo'
        () => "^(?:.*\\/)?",
        "*"
      ],
      // starting
      [
        // there will be no leading '/'
        //   (which has been replaced by section "leading slash")
        // If starts with '**', adding a '^' to the regular expression also works
        DIRECT,
        (source, pattern) => {
          if (!source || source[0] === "^") {
            return source;
          }
          const anchor = !REGEX_INNER_SLASH.test(pattern) ? "(?:^|\\/)" : "^";
          return anchor + source;
        }
      ],
      // two globstars
      [
        // Use lookahead assertions so that we could match more than one `'/**'`
        /\\\/\\\*\\\*(?=\\\/|$)/g,
        // Zero, one or several directories
        // should not use '*', or it will be replaced by the next replacer
        // Check if it is not the last `'/**'`
        (_, index, str2) => index + 6 < str2.length ? str2.slice(index + 6) === "\\/" ? "(?:\\/[^\\/]+)+" : "(?:\\/[^\\/]+)*" : "\\/.+",
        "*"
      ],
      // normal intermediate wildcards
      [
        // Never replace escaped '*'
        // ignore rule '\*' will match the path '*'
        // 'abc.*/' -> go
        // 'abc.*'  -> skip this rule,
        //    coz trailing single wildcard will be handed by [trailing wildcard]
        /(^|[^\\]+)(\\\*)+(?=.+)/g,
        // '*.js' matches '.js'
        // '*.js' doesn't match 'abc'
        (_, p1, p2) => {
          const unescaped = p2.replace(/\\\*/g, "[^\\/]*");
          return p1 + unescaped;
        },
        "*"
      ],
      // trailing wildcard, held apart from a literal star
      [
        // The step above leaves a trailing `*` alone, so a single `\*` is all that
        //   can be left at the end here. Whether it is a wildcard or a literal
        //   turns on the backslashes the user put in front of it: the escaper has
        //   since doubled every one, so what stands here is those `2N` doubled
        //   backslashes and then the star's own escape. An even number of the
        //   original `N` leaves the star unescaped -- a wildcard -- and an odd
        //   number escapes it -- a literal. This runs while the two are still
        //   distinct, before the unescape steps below collapse the literal onto
        //   the very `\*` a wildcard leaves behind.
        /(^|[^\\])((?:\\\\)*)\\\*$/,
        (match2, p1, p2) => (
          // `p2` holds the doubled user backslashes; half of them is `N`.
          p2.length / 2 % 2 === 0 ? p1 + p2 + TRAILING_WILDCARD : match2
        ),
        "*"
      ],
      [
        // unescape, revert step 3 except for back slash
        // For example, if a user escape a '\\*',
        // after step 3, the result will be '\\\\\\*'
        /\\\\\\(?=[$.|*+(){^])/g,
        () => ESCAPE,
        ESCAPE + ESCAPE
      ],
      [
        // '\\\\' -> '\\'
        /\\\\/g,
        () => ESCAPE,
        ESCAPE + ESCAPE
      ],
      [
        // Every real bracket expression -- POSIX classes included -- has already
        //   been held aside by `extractBrackets`, so the only `[` left in the
        //   pattern is an escaped, literal one.
        // `\` is escaped by step 3
        /\\\[([^\]/]*?)(\\*)($|\])/g,
        // '\\[bar]' -> '\\\\[bar\\]'
        (match2, range2, endEscape, close) => `\\[${range2}${cleanRangeBackSlash(endEscape)}${close}`,
        "["
      ],
      // ending
      [
        // 'js' will not match 'js.'
        // 'ab' will not match 'abc'
        DIRECT,
        // WTF!
        // https://git-scm.com/docs/gitignore
        // changes in [2.22.1](https://git-scm.com/docs/gitignore/2.22.1)
        // which re-fixes #24, #38
        // > If there is a separator at the end of the pattern then the pattern
        // > will only match directories, otherwise the pattern can match both
        // > files and directories.
        // 'js*' will not match 'a.js'
        // 'js/' will not match 'a.js'
        // 'js' will match 'a.js' and 'a.js/'
        (source) => {
          const last = source[source.length - 1];
          if (!last || last === TRAILING_WILDCARD) {
            return source;
          }
          return last === SLASH ? `${source}$` : `${source}(?=$|\\/$)`;
        }
      ]
    ];
    var REGEX_REPLACE_TRAILING_WILDCARD = /(^|\\\/)?\uE000$/;
    var MODE_IGNORE = "regex";
    var MODE_CHECK_IGNORE = "checkRegex";
    var UNDERSCORE = "_";
    var TRAILING_WILD_CARD_REPLACERS = {
      [MODE_IGNORE](_, p1) {
        const prefix = p1 ? `${p1}[^/]+` : "[^/]*";
        return `${prefix}(?=$|\\/$)`;
      },
      [MODE_CHECK_IGNORE](_, p1) {
        const prefix = p1 ? `${p1}[^/]*` : "[^/]*";
        return `${prefix}(?=$|\\/$)`;
      }
    };
    var WILDCARD = "[^\\/]*";
    var pinWildcards = (source) => {
      if (source.indexOf(WILDCARD) < 0) {
        return source;
      }
      const tokens = [];
      const { length } = source;
      let index = 0;
      while (index < length) {
        const char = source[index];
        if (source.startsWith(WILDCARD, index)) {
          tokens.push({ wildcard: true });
          index += WILDCARD.length;
        } else if (char === "[") {
          let end = index + 1;
          if (source[end] === "^") {
            end++;
          }
          if (source[end] === "]") {
            end++;
          }
          while (end < length && source[end] !== "]") {
            end += source[end] === ESCAPE ? 2 : 1;
          }
          end++;
          tokens.push({ single: source.slice(index, end) });
          index = end;
        } else if (char === ESCAPE) {
          tokens.push({ single: source.slice(index, index + 2) });
          index += 2;
        } else if (char === "(") {
          let depth = 0;
          let end = index;
          do {
            if (source[end] === ESCAPE) {
              end++;
            } else if (source[end] === "(") {
              depth++;
            } else if (source[end] === ")") {
              depth--;
            }
            end++;
          } while (end < length && depth > 0);
          if ("*+?".indexOf(source[end]) >= 0) {
            end++;
          }
          tokens.push({ boundary: source.slice(index, end) });
          index = end;
        } else if (char === "^" || char === "$") {
          tokens.push({ boundary: char });
          index++;
        } else {
          tokens.push({ single: char });
          index++;
        }
      }
      let out = EMPTY;
      let run = [];
      const flush = () => {
        let lastWildcard;
        run.forEach((token, at) => {
          if (token.wildcard) {
            lastWildcard = at;
          }
        });
        run.forEach((token, at) => {
          if (!token.wildcard) {
            out += token.single;
            return;
          }
          out += at === lastWildcard ? WILDCARD : `(?:(?!${run[at + 1].single})[^\\/])*`;
        });
        run = [];
      };
      tokens.forEach((token) => {
        if (token.boundary === void 0) {
          run.push(token);
          return;
        }
        flush();
        out += token.boundary;
      });
      flush();
      return out;
    };
    var makeRegexPrefix = (pattern) => {
      const { source, sources } = extractBrackets(pattern);
      const replaced = REPLACERS.reduce(
        // A pass whose matcher finds nothing hands back the very string it was
        //   given, so asking first costs a search and saves a rewrite. Ten of the
        //   fifteen passes never fire for a typical .gitignore line, and between
        //   them they were 45% of this chain.
        (prev, [matcher, replacer, required]) => {
          if (matcher === DIRECT) {
            return replacer(prev, pattern);
          }
          if (required !== UNDEFINED && prev.indexOf(required) < 0) {
            return prev;
          }
          return matcher.test(prev) ? prev.replace(matcher, replacer.bind(pattern)) : prev;
        },
        source
      );
      return sources.length ? replaced.replace(
        REGEX_RESTORE_PLACEHOLDER,
        (match2, index) => sources[index]
      ) : replaced;
    };
    var matchesBasename = (body) => {
      const index = body.indexOf(SLASH);
      return index < 0 || index === body.length - 1;
    };
    var basenameOf = (path4) => {
      const end = path4.length - 1;
      const index = path4.lastIndexOf(
        SLASH,
        path4[end] === SLASH ? end - 1 : end
      );
      return index < 0 ? path4 : path4.slice(index + 1);
    };
    var parentOf = (path4) => {
      if (path4.charCodeAt(0) === SLASH_CODE || path4.indexOf(DOUBLE_SLASH) >= 0) {
        const slices = path4.split(SLASH).filter(Boolean);
        slices.pop();
        return slices.length ? slices.join(SLASH) + SLASH : EMPTY;
      }
      const end = path4.length - 1;
      const cut = path4.lastIndexOf(
        SLASH,
        path4.charCodeAt(end) === SLASH_CODE ? end - 1 : end
      );
      return cut < 0 ? EMPTY : path4.slice(0, cut + 1);
    };
    var isString = (subject) => typeof subject === "string";
    var checkPattern = (pattern) => pattern && isString(pattern) && !REGEX_TEST_BLANK_LINE.test(pattern) && !REGEX_INVALID_TRAILING_BACKSLASH.test(pattern) && pattern.indexOf("#") !== 0;
    var splitPattern = (pattern) => pattern.split(REGEX_SPLITALL_CRLF).filter(Boolean);
    var IgnoreRule = class {
      constructor(pattern, mark, body, ignoreCase, negative, prefix) {
        this.pattern = pattern;
        this.mark = mark;
        this.negative = negative;
        define(this, "body", body);
        define(this, "ignoreCase", ignoreCase);
        define(this, "regexPrefix", prefix);
      }
      // Worked out on first use and kept behind an own property, the way `regex`
      //   caches itself in `_regex`. Deciding it in the constructor instead would
      //   add a fourth `defineProperty` to every rule ever built, which cost 4% of
      //   every compile -- including the compiles of rules that are never matched
      //   against anything.
      get _basenameOnly() {
        return define(this, "_basenameOnly", matchesBasename(this.body));
      }
      get regex() {
        const key = UNDERSCORE + MODE_IGNORE;
        if (this[key]) {
          return this[key];
        }
        return this._make(MODE_IGNORE, key);
      }
      get checkRegex() {
        const key = UNDERSCORE + MODE_CHECK_IGNORE;
        if (this[key]) {
          return this[key];
        }
        return this._make(MODE_CHECK_IGNORE, key);
      }
      _make(mode, key) {
        const str2 = pinWildcards(this.regexPrefix.replace(
          REGEX_REPLACE_TRAILING_WILDCARD,
          // It does not need to bind pattern
          TRAILING_WILD_CARD_REPLACERS[mode]
        ));
        const regex2 = this.ignoreCase ? new RegExp(str2, "i") : new RegExp(str2);
        return define(this, key, regex2);
      }
    };
    var createRule = ({
      pattern,
      mark
    }, ignoreCase) => {
      let negative = false;
      let body = pattern;
      if (body.indexOf("!") === 0) {
        negative = true;
        body = body.substr(1);
      }
      body = body.replace(REGEX_REPLACE_LEADING_EXCAPED_EXCLAMATION, "!").replace(REGEX_REPLACE_LEADING_EXCAPED_HASH, "#");
      const regexPrefix = makeRegexPrefix(body);
      return new IgnoreRule(
        pattern,
        mark,
        body,
        ignoreCase,
        negative,
        regexPrefix
      );
    };
    var RuleManager = class {
      constructor(ignoreCase) {
        this._ignoreCase = ignoreCase;
        this._rules = [];
        this._basenameCount = 0;
      }
      _add(pattern) {
        if (pattern && pattern[KEY_IGNORE]) {
          this._rules = this._rules.concat(pattern._rules._rules);
          this._basenameCount += pattern._rules._basenameCount;
          this._added = true;
          return;
        }
        if (isString(pattern)) {
          pattern = {
            pattern
          };
        }
        if (checkPattern(pattern.pattern)) {
          const rule = createRule(pattern, this._ignoreCase);
          this._added = true;
          this._rules.push(rule);
          if (matchesBasename(rule.body)) {
            this._basenameCount++;
          }
        }
      }
      // @param {Array<string> | string | Ignore} pattern
      add(pattern) {
        this._added = false;
        makeArray(
          isString(pattern) ? splitPattern(pattern) : pattern
        ).forEach(this._add, this);
        return this._added;
      }
      // Test one single path without recursively checking parent directories
      //
      // - checkUnignored `boolean` whether should check if the path is unignored,
      //   setting `checkUnignored` to `false` could reduce additional
      //   path matching.
      // - check `string` either `MODE_IGNORE` or `MODE_CHECK_IGNORE`
      // @returns {TestResult} true if a file is ignored
      test(path4, checkUnignored, mode) {
        let ignored = false;
        let unignored = false;
        let matchedRule;
        const rules = this._rules;
        const { length } = rules;
        const shortcut = this._basenameCount * 2 >= length;
        const basename4 = shortcut ? basenameOf(path4) : path4;
        for (let index = 0; index < length; index++) {
          const rule = rules[index];
          const { negative } = rule;
          const skip = unignored === negative && ignored !== unignored || negative && !ignored && !unignored && !checkUnignored;
          if (!skip && rule[mode].test(
            shortcut && rule._basenameOnly ? basename4 : path4
          )) {
            ignored = !negative;
            unignored = negative;
            matchedRule = negative ? UNDEFINED : rule;
          }
        }
        const ret = {
          ignored,
          unignored
        };
        if (matchedRule) {
          ret.rule = matchedRule;
        }
        return ret;
      }
    };
    var throwError = (message, Ctor) => {
      throw new Ctor(message);
    };
    var checkPath = (path4, originalPath, doThrow) => {
      if (!isString(path4)) {
        return doThrow(
          `path must be a string, but got \`${originalPath}\``,
          TypeError
        );
      }
      if (!path4) {
        return doThrow(`path must not be empty`, TypeError);
      }
      if (checkPath.isNotRelative(path4)) {
        const r = "`path.relative()`d";
        return doThrow(
          `path should be a ${r} string, but got "${originalPath}"`,
          RangeError
        );
      }
      return true;
    };
    var isNotRelative = (path4) => {
      const first = path4.charCodeAt(0);
      if (first === SLASH_CODE) {
        return true;
      }
      if (first !== DOT_CODE) {
        return false;
      }
      if (path4.length === 1) {
        return true;
      }
      const second = path4.charCodeAt(1);
      if (second === SLASH_CODE) {
        return true;
      }
      if (second !== DOT_CODE) {
        return false;
      }
      return path4.length === 2 || path4.charCodeAt(2) === SLASH_CODE;
    };
    checkPath.isNotRelative = isNotRelative;
    checkPath.convert = (p) => p;
    var Ignore = class {
      constructor({
        ignorecase = true,
        ignoreCase = ignorecase,
        allowRelativePaths = false
      } = {}) {
        define(this, KEY_IGNORE, true);
        this._rules = new RuleManager(ignoreCase);
        this._strictPathCheck = !allowRelativePaths;
        this._initCache();
      }
      _initCache() {
        this._ignoreCache = /* @__PURE__ */ Object.create(null);
        this._testCache = /* @__PURE__ */ Object.create(null);
      }
      add(pattern) {
        if (this._rules.add(pattern)) {
          this._initCache();
        }
        return this;
      }
      // legacy
      addPattern(pattern) {
        return this.add(pattern);
      }
      // @returns {TestResult}
      _test(originalPath, cache, checkUnignored) {
        const path4 = originalPath && checkPath.convert(originalPath);
        checkPath(
          path4,
          originalPath,
          this._strictPathCheck ? throwError : RETURN_FALSE
        );
        return this._t(path4, cache, checkUnignored);
      }
      checkIgnore(path4) {
        if (path4.charCodeAt(path4.length - 1) !== SLASH_CODE) {
          return this.test(path4);
        }
        const parentPath = parentOf(path4);
        if (parentPath) {
          const parent = this._t(parentPath, this._testCache, true);
          if (parent.ignored) {
            return parent;
          }
        }
        return this._rules.test(path4, false, MODE_CHECK_IGNORE);
      }
      _t(path4, cache, checkUnignored) {
        if (path4 in cache) {
          return cache[path4];
        }
        const parentPath = parentOf(path4);
        const parent = parentPath ? this._t(parentPath, cache, checkUnignored) : UNDEFINED;
        return cache[path4] = parent && parent.ignored ? parent : this._rules.test(path4, checkUnignored, MODE_IGNORE);
      }
      ignores(path4) {
        return this._test(path4, this._ignoreCache, false).ignored;
      }
      createFilter() {
        return (path4) => !this.ignores(path4);
      }
      filter(paths) {
        return makeArray(paths).filter(this.createFilter());
      }
      // @returns {TestResult}
      test(path4) {
        return this._test(path4, this._testCache, true);
      }
    };
    var factory = (options) => new Ignore(options);
    var isPathValid = (path4) => checkPath(path4 && checkPath.convert(path4), path4, RETURN_FALSE);
    var setupWindows = () => {
      const makePosix = (str2) => /^\\\\\?\\/.test(str2) || /["<>|\u0000-\u001F]+/u.test(str2) ? str2 : str2.replace(/\\/g, "/");
      checkPath.convert = makePosix;
      const REGEX_TEST_WINDOWS_PATH_ABSOLUTE = /^[a-z]:\//i;
      checkPath.isNotRelative = (path4) => REGEX_TEST_WINDOWS_PATH_ABSOLUTE.test(path4) || isNotRelative(path4);
    };
    if (
      // Detect `process` so that it can run in browsers.
      typeof process !== "undefined" && process.platform === "win32"
    ) {
      setupWindows();
    }
    module.exports = factory;
    factory.default = factory;
    module.exports.isPathValid = isPathValid;
    define(module.exports, Symbol.for("setupWindows"), setupWindows);
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/graceful-fs/polyfills.js
var require_polyfills = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/graceful-fs/polyfills.js"(exports, module) {
    var constants6 = __require("constants");
    var origCwd = process.cwd;
    var cwd = null;
    var platform2 = process.env.GRACEFUL_FS_PLATFORM || process.platform;
    process.cwd = function() {
      if (!cwd)
        cwd = origCwd.call(process);
      return cwd;
    };
    try {
      process.cwd();
    } catch (er) {
    }
    if (typeof process.chdir === "function") {
      chdir = process.chdir;
      process.chdir = function(d) {
        cwd = null;
        chdir.call(process, d);
      };
      if (Object.setPrototypeOf) Object.setPrototypeOf(process.chdir, chdir);
    }
    var chdir;
    module.exports = patch;
    function patch(fs) {
      if (constants6.hasOwnProperty("O_SYMLINK") && process.version.match(/^v0\.6\.[0-2]|^v0\.5\./)) {
        patchLchmod(fs);
      }
      if (!fs.lutimes) {
        patchLutimes(fs);
      }
      fs.chown = chownFix(fs.chown);
      fs.fchown = chownFix(fs.fchown);
      fs.lchown = chownFix(fs.lchown);
      fs.chmod = chmodFix(fs.chmod);
      fs.fchmod = chmodFix(fs.fchmod);
      fs.lchmod = chmodFix(fs.lchmod);
      fs.chownSync = chownFixSync(fs.chownSync);
      fs.fchownSync = chownFixSync(fs.fchownSync);
      fs.lchownSync = chownFixSync(fs.lchownSync);
      fs.chmodSync = chmodFixSync(fs.chmodSync);
      fs.fchmodSync = chmodFixSync(fs.fchmodSync);
      fs.lchmodSync = chmodFixSync(fs.lchmodSync);
      fs.stat = statFix(fs.stat);
      fs.fstat = statFix(fs.fstat);
      fs.lstat = statFix(fs.lstat);
      fs.statSync = statFixSync(fs.statSync);
      fs.fstatSync = statFixSync(fs.fstatSync);
      fs.lstatSync = statFixSync(fs.lstatSync);
      if (fs.chmod && !fs.lchmod) {
        fs.lchmod = function(path4, mode, cb) {
          if (cb) process.nextTick(cb);
        };
        fs.lchmodSync = function() {
        };
      }
      if (fs.chown && !fs.lchown) {
        fs.lchown = function(path4, uid, gid, cb) {
          if (cb) process.nextTick(cb);
        };
        fs.lchownSync = function() {
        };
      }
      if (platform2 === "win32") {
        fs.rename = typeof fs.rename !== "function" ? fs.rename : (function(fs$rename) {
          function rename(from, to, cb) {
            var start = Date.now();
            var backoff = 0;
            fs$rename(from, to, function CB(er) {
              if (er && (er.code === "EACCES" || er.code === "EPERM" || er.code === "EBUSY") && Date.now() - start < 6e4) {
                setTimeout(function() {
                  fs.stat(to, function(stater, st) {
                    if (stater && stater.code === "ENOENT")
                      fs$rename(from, to, CB);
                    else
                      cb(er);
                  });
                }, backoff);
                if (backoff < 100)
                  backoff += 10;
                return;
              }
              if (cb) cb(er);
            });
          }
          if (Object.setPrototypeOf) Object.setPrototypeOf(rename, fs$rename);
          return rename;
        })(fs.rename);
      }
      fs.read = typeof fs.read !== "function" ? fs.read : (function(fs$read) {
        function read(fd, buffer, offset, length, position, callback_) {
          var callback;
          if (callback_ && typeof callback_ === "function") {
            var eagCounter = 0;
            callback = function(er, _, __) {
              if (er && er.code === "EAGAIN" && eagCounter < 10) {
                eagCounter++;
                return fs$read.call(fs, fd, buffer, offset, length, position, callback);
              }
              callback_.apply(this, arguments);
            };
          }
          return fs$read.call(fs, fd, buffer, offset, length, position, callback);
        }
        if (Object.setPrototypeOf) Object.setPrototypeOf(read, fs$read);
        return read;
      })(fs.read);
      fs.readSync = typeof fs.readSync !== "function" ? fs.readSync : /* @__PURE__ */ (function(fs$readSync) {
        return function(fd, buffer, offset, length, position) {
          var eagCounter = 0;
          while (true) {
            try {
              return fs$readSync.call(fs, fd, buffer, offset, length, position);
            } catch (er) {
              if (er.code === "EAGAIN" && eagCounter < 10) {
                eagCounter++;
                continue;
              }
              throw er;
            }
          }
        };
      })(fs.readSync);
      function patchLchmod(fs2) {
        fs2.lchmod = function(path4, mode, callback) {
          fs2.open(
            path4,
            constants6.O_WRONLY | constants6.O_SYMLINK,
            mode,
            function(err, fd) {
              if (err) {
                if (callback) callback(err);
                return;
              }
              fs2.fchmod(fd, mode, function(err2) {
                fs2.close(fd, function(err22) {
                  if (callback) callback(err2 || err22);
                });
              });
            }
          );
        };
        fs2.lchmodSync = function(path4, mode) {
          var fd = fs2.openSync(path4, constants6.O_WRONLY | constants6.O_SYMLINK, mode);
          var threw = true;
          var ret;
          try {
            ret = fs2.fchmodSync(fd, mode);
            threw = false;
          } finally {
            if (threw) {
              try {
                fs2.closeSync(fd);
              } catch (er) {
              }
            } else {
              fs2.closeSync(fd);
            }
          }
          return ret;
        };
      }
      function patchLutimes(fs2) {
        if (constants6.hasOwnProperty("O_SYMLINK") && fs2.futimes) {
          fs2.lutimes = function(path4, at, mt, cb) {
            fs2.open(path4, constants6.O_SYMLINK, function(er, fd) {
              if (er) {
                if (cb) cb(er);
                return;
              }
              fs2.futimes(fd, at, mt, function(er2) {
                fs2.close(fd, function(er22) {
                  if (cb) cb(er2 || er22);
                });
              });
            });
          };
          fs2.lutimesSync = function(path4, at, mt) {
            var fd = fs2.openSync(path4, constants6.O_SYMLINK);
            var ret;
            var threw = true;
            try {
              ret = fs2.futimesSync(fd, at, mt);
              threw = false;
            } finally {
              if (threw) {
                try {
                  fs2.closeSync(fd);
                } catch (er) {
                }
              } else {
                fs2.closeSync(fd);
              }
            }
            return ret;
          };
        } else if (fs2.futimes) {
          fs2.lutimes = function(_a2, _b, _c, cb) {
            if (cb) process.nextTick(cb);
          };
          fs2.lutimesSync = function() {
          };
        }
      }
      function chmodFix(orig) {
        if (!orig) return orig;
        return function(target, mode, cb) {
          return orig.call(fs, target, mode, function(er) {
            if (chownErOk(er)) er = null;
            if (cb) cb.apply(this, arguments);
          });
        };
      }
      function chmodFixSync(orig) {
        if (!orig) return orig;
        return function(target, mode) {
          try {
            return orig.call(fs, target, mode);
          } catch (er) {
            if (!chownErOk(er)) throw er;
          }
        };
      }
      function chownFix(orig) {
        if (!orig) return orig;
        return function(target, uid, gid, cb) {
          return orig.call(fs, target, uid, gid, function(er) {
            if (chownErOk(er)) er = null;
            if (cb) cb.apply(this, arguments);
          });
        };
      }
      function chownFixSync(orig) {
        if (!orig) return orig;
        return function(target, uid, gid) {
          try {
            return orig.call(fs, target, uid, gid);
          } catch (er) {
            if (!chownErOk(er)) throw er;
          }
        };
      }
      function statFix(orig) {
        if (!orig) return orig;
        return function(target, options, cb) {
          if (typeof options === "function") {
            cb = options;
            options = null;
          }
          function callback(er, stats) {
            if (stats) {
              if (stats.uid < 0) stats.uid += 4294967296;
              if (stats.gid < 0) stats.gid += 4294967296;
            }
            if (cb) cb.apply(this, arguments);
          }
          return options ? orig.call(fs, target, options, callback) : orig.call(fs, target, callback);
        };
      }
      function statFixSync(orig) {
        if (!orig) return orig;
        return function(target, options) {
          var stats = options ? orig.call(fs, target, options) : orig.call(fs, target);
          if (stats) {
            if (stats.uid < 0) stats.uid += 4294967296;
            if (stats.gid < 0) stats.gid += 4294967296;
          }
          return stats;
        };
      }
      function chownErOk(er) {
        if (!er)
          return true;
        if (er.code === "ENOSYS")
          return true;
        var nonroot = !process.getuid || process.getuid() !== 0;
        if (nonroot) {
          if (er.code === "EINVAL" || er.code === "EPERM")
            return true;
        }
        return false;
      }
    }
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/graceful-fs/legacy-streams.js
var require_legacy_streams = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/graceful-fs/legacy-streams.js"(exports, module) {
    var Stream = __require("stream").Stream;
    module.exports = legacy;
    function legacy(fs) {
      return {
        ReadStream,
        WriteStream
      };
      function ReadStream(path4, options) {
        if (!(this instanceof ReadStream)) return new ReadStream(path4, options);
        Stream.call(this);
        var self = this;
        this.path = path4;
        this.fd = null;
        this.readable = true;
        this.paused = false;
        this.flags = "r";
        this.mode = 438;
        this.bufferSize = 64 * 1024;
        options = options || {};
        var keys = Object.keys(options);
        for (var index = 0, length = keys.length; index < length; index++) {
          var key = keys[index];
          this[key] = options[key];
        }
        if (this.encoding) this.setEncoding(this.encoding);
        if (this.start !== void 0) {
          if ("number" !== typeof this.start) {
            throw TypeError("start must be a Number");
          }
          if (this.end === void 0) {
            this.end = Infinity;
          } else if ("number" !== typeof this.end) {
            throw TypeError("end must be a Number");
          }
          if (this.start > this.end) {
            throw new Error("start must be <= end");
          }
          this.pos = this.start;
        }
        if (this.fd !== null) {
          process.nextTick(function() {
            self._read();
          });
          return;
        }
        fs.open(this.path, this.flags, this.mode, function(err, fd) {
          if (err) {
            self.emit("error", err);
            self.readable = false;
            return;
          }
          self.fd = fd;
          self.emit("open", fd);
          self._read();
        });
      }
      function WriteStream(path4, options) {
        if (!(this instanceof WriteStream)) return new WriteStream(path4, options);
        Stream.call(this);
        this.path = path4;
        this.fd = null;
        this.writable = true;
        this.flags = "w";
        this.encoding = "binary";
        this.mode = 438;
        this.bytesWritten = 0;
        options = options || {};
        var keys = Object.keys(options);
        for (var index = 0, length = keys.length; index < length; index++) {
          var key = keys[index];
          this[key] = options[key];
        }
        if (this.start !== void 0) {
          if ("number" !== typeof this.start) {
            throw TypeError("start must be a Number");
          }
          if (this.start < 0) {
            throw new Error("start must be >= zero");
          }
          this.pos = this.start;
        }
        this.busy = false;
        this._queue = [];
        if (this.fd === null) {
          this._open = fs.open;
          this._queue.push([this._open, this.path, this.flags, this.mode, void 0]);
          this.flush();
        }
      }
    }
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/graceful-fs/clone.js
var require_clone = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/graceful-fs/clone.js"(exports, module) {
    "use strict";
    module.exports = clone;
    var getPrototypeOf = Object.getPrototypeOf || function(obj) {
      return obj.__proto__;
    };
    function clone(obj) {
      if (obj === null || typeof obj !== "object")
        return obj;
      if (obj instanceof Object)
        var copy = { __proto__: getPrototypeOf(obj) };
      else
        var copy = /* @__PURE__ */ Object.create(null);
      Object.getOwnPropertyNames(obj).forEach(function(key) {
        Object.defineProperty(copy, key, Object.getOwnPropertyDescriptor(obj, key));
      });
      return copy;
    }
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/graceful-fs/graceful-fs.js
var require_graceful_fs = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/graceful-fs/graceful-fs.js"(exports, module) {
    var fs = __require("fs");
    var polyfills = require_polyfills();
    var legacy = require_legacy_streams();
    var clone = require_clone();
    var util = __require("util");
    var gracefulQueue;
    var previousSymbol;
    if (typeof Symbol === "function" && typeof Symbol.for === "function") {
      gracefulQueue = Symbol.for("graceful-fs.queue");
      previousSymbol = Symbol.for("graceful-fs.previous");
    } else {
      gracefulQueue = "___graceful-fs.queue";
      previousSymbol = "___graceful-fs.previous";
    }
    function noop() {
    }
    function publishQueue(context, queue2) {
      Object.defineProperty(context, gracefulQueue, {
        get: function() {
          return queue2;
        }
      });
    }
    var debug = noop;
    if (util.debuglog)
      debug = util.debuglog("gfs4");
    else if (/\bgfs4\b/i.test(process.env.NODE_DEBUG || ""))
      debug = function() {
        var m = util.format.apply(util, arguments);
        m = "GFS4: " + m.split(/\n/).join("\nGFS4: ");
        console.error(m);
      };
    if (!fs[gracefulQueue]) {
      queue = global[gracefulQueue] || [];
      publishQueue(fs, queue);
      fs.close = (function(fs$close) {
        function close(fd, cb) {
          return fs$close.call(fs, fd, function(err) {
            if (!err) {
              resetQueue();
            }
            if (typeof cb === "function")
              cb.apply(this, arguments);
          });
        }
        Object.defineProperty(close, previousSymbol, {
          value: fs$close
        });
        return close;
      })(fs.close);
      fs.closeSync = (function(fs$closeSync) {
        function closeSync2(fd) {
          fs$closeSync.apply(fs, arguments);
          resetQueue();
        }
        Object.defineProperty(closeSync2, previousSymbol, {
          value: fs$closeSync
        });
        return closeSync2;
      })(fs.closeSync);
      if (/\bgfs4\b/i.test(process.env.NODE_DEBUG || "")) {
        process.on("exit", function() {
          debug(fs[gracefulQueue]);
          __require("assert").equal(fs[gracefulQueue].length, 0);
        });
      }
    }
    var queue;
    if (!global[gracefulQueue]) {
      publishQueue(global, fs[gracefulQueue]);
    }
    module.exports = patch(clone(fs));
    if (process.env.TEST_GRACEFUL_FS_GLOBAL_PATCH && !fs.__patched) {
      module.exports = patch(fs);
      fs.__patched = true;
    }
    function patch(fs2) {
      polyfills(fs2);
      fs2.gracefulify = patch;
      fs2.createReadStream = createReadStream2;
      fs2.createWriteStream = createWriteStream3;
      var fs$readFile = fs2.readFile;
      fs2.readFile = readFile2;
      function readFile2(path4, options, cb) {
        if (typeof options === "function")
          cb = options, options = null;
        return go$readFile(path4, options, cb);
        function go$readFile(path5, options2, cb2, startTime) {
          return fs$readFile(path5, options2, function(err) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$readFile, [path5, options2, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      var fs$writeFile = fs2.writeFile;
      fs2.writeFile = writeFile;
      function writeFile(path4, data, options, cb) {
        if (typeof options === "function")
          cb = options, options = null;
        return go$writeFile(path4, data, options, cb);
        function go$writeFile(path5, data2, options2, cb2, startTime) {
          return fs$writeFile(path5, data2, options2, function(err) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$writeFile, [path5, data2, options2, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      var fs$appendFile = fs2.appendFile;
      if (fs$appendFile)
        fs2.appendFile = appendFile;
      function appendFile(path4, data, options, cb) {
        if (typeof options === "function")
          cb = options, options = null;
        return go$appendFile(path4, data, options, cb);
        function go$appendFile(path5, data2, options2, cb2, startTime) {
          return fs$appendFile(path5, data2, options2, function(err) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$appendFile, [path5, data2, options2, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      var fs$copyFile = fs2.copyFile;
      if (fs$copyFile)
        fs2.copyFile = copyFile;
      function copyFile(src, dest, flags, cb) {
        if (typeof flags === "function") {
          cb = flags;
          flags = 0;
        }
        return go$copyFile(src, dest, flags, cb);
        function go$copyFile(src2, dest2, flags2, cb2, startTime) {
          return fs$copyFile(src2, dest2, flags2, function(err) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$copyFile, [src2, dest2, flags2, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      var fs$readdir = fs2.readdir;
      fs2.readdir = readdir2;
      var noReaddirOptionVersions = /^v[0-5]\./;
      function readdir2(path4, options, cb) {
        if (typeof options === "function")
          cb = options, options = null;
        var go$readdir = noReaddirOptionVersions.test(process.version) ? function go$readdir2(path5, options2, cb2, startTime) {
          return fs$readdir(path5, fs$readdirCallback(
            path5,
            options2,
            cb2,
            startTime
          ));
        } : function go$readdir2(path5, options2, cb2, startTime) {
          return fs$readdir(path5, options2, fs$readdirCallback(
            path5,
            options2,
            cb2,
            startTime
          ));
        };
        return go$readdir(path4, options, cb);
        function fs$readdirCallback(path5, options2, cb2, startTime) {
          return function(err, files) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([
                go$readdir,
                [path5, options2, cb2],
                err,
                startTime || Date.now(),
                Date.now()
              ]);
            else {
              if (files && files.sort)
                files.sort();
              if (typeof cb2 === "function")
                cb2.call(this, err, files);
            }
          };
        }
      }
      if (process.version.substr(0, 4) === "v0.8") {
        var legStreams = legacy(fs2);
        ReadStream = legStreams.ReadStream;
        WriteStream = legStreams.WriteStream;
      }
      var fs$ReadStream = fs2.ReadStream;
      if (fs$ReadStream) {
        ReadStream.prototype = Object.create(fs$ReadStream.prototype);
        ReadStream.prototype.open = ReadStream$open;
      }
      var fs$WriteStream = fs2.WriteStream;
      if (fs$WriteStream) {
        WriteStream.prototype = Object.create(fs$WriteStream.prototype);
        WriteStream.prototype.open = WriteStream$open;
      }
      Object.defineProperty(fs2, "ReadStream", {
        get: function() {
          return ReadStream;
        },
        set: function(val) {
          ReadStream = val;
        },
        enumerable: true,
        configurable: true
      });
      Object.defineProperty(fs2, "WriteStream", {
        get: function() {
          return WriteStream;
        },
        set: function(val) {
          WriteStream = val;
        },
        enumerable: true,
        configurable: true
      });
      var FileReadStream = ReadStream;
      Object.defineProperty(fs2, "FileReadStream", {
        get: function() {
          return FileReadStream;
        },
        set: function(val) {
          FileReadStream = val;
        },
        enumerable: true,
        configurable: true
      });
      var FileWriteStream = WriteStream;
      Object.defineProperty(fs2, "FileWriteStream", {
        get: function() {
          return FileWriteStream;
        },
        set: function(val) {
          FileWriteStream = val;
        },
        enumerable: true,
        configurable: true
      });
      function ReadStream(path4, options) {
        if (this instanceof ReadStream)
          return fs$ReadStream.apply(this, arguments), this;
        else
          return ReadStream.apply(Object.create(ReadStream.prototype), arguments);
      }
      function ReadStream$open() {
        var that = this;
        open3(that.path, that.flags, that.mode, function(err, fd) {
          if (err) {
            if (that.autoClose)
              that.destroy();
            that.emit("error", err);
          } else {
            that.fd = fd;
            that.emit("open", fd);
            that.read();
          }
        });
      }
      function WriteStream(path4, options) {
        if (this instanceof WriteStream)
          return fs$WriteStream.apply(this, arguments), this;
        else
          return WriteStream.apply(Object.create(WriteStream.prototype), arguments);
      }
      function WriteStream$open() {
        var that = this;
        open3(that.path, that.flags, that.mode, function(err, fd) {
          if (err) {
            that.destroy();
            that.emit("error", err);
          } else {
            that.fd = fd;
            that.emit("open", fd);
          }
        });
      }
      function createReadStream2(path4, options) {
        return new fs2.ReadStream(path4, options);
      }
      function createWriteStream3(path4, options) {
        return new fs2.WriteStream(path4, options);
      }
      var fs$open = fs2.open;
      fs2.open = open3;
      function open3(path4, flags, mode, cb) {
        if (typeof mode === "function")
          cb = mode, mode = null;
        return go$open(path4, flags, mode, cb);
        function go$open(path5, flags2, mode2, cb2, startTime) {
          return fs$open(path5, flags2, mode2, function(err, fd) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$open, [path5, flags2, mode2, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      return fs2;
    }
    function enqueue(elem) {
      debug("ENQUEUE", elem[0].name, elem[1]);
      fs[gracefulQueue].push(elem);
      retry();
    }
    var retryTimer;
    function resetQueue() {
      var now = Date.now();
      for (var i = 0; i < fs[gracefulQueue].length; ++i) {
        if (fs[gracefulQueue][i].length > 2) {
          fs[gracefulQueue][i][3] = now;
          fs[gracefulQueue][i][4] = now;
        }
      }
      retry();
    }
    function retry() {
      clearTimeout(retryTimer);
      retryTimer = void 0;
      if (fs[gracefulQueue].length === 0)
        return;
      var elem = fs[gracefulQueue].shift();
      var fn = elem[0];
      var args = elem[1];
      var err = elem[2];
      var startTime = elem[3];
      var lastTime = elem[4];
      if (startTime === void 0) {
        debug("RETRY", fn.name, args);
        fn.apply(null, args);
      } else if (Date.now() - startTime >= 6e4) {
        debug("TIMEOUT", fn.name, args);
        var cb = args.pop();
        if (typeof cb === "function")
          cb.call(null, err);
      } else {
        var sinceAttempt = Date.now() - lastTime;
        var sinceStart = Math.max(lastTime - startTime, 1);
        var desiredDelay = Math.min(sinceStart * 1.2, 100);
        if (sinceAttempt >= desiredDelay) {
          debug("RETRY", fn.name, args);
          fn.apply(null, args.concat([startTime]));
        } else {
          fs[gracefulQueue].push(elem);
        }
      }
      if (retryTimer === void 0) {
        retryTimer = setTimeout(retry, 0);
      }
    }
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/node_modules/retry/lib/retry_operation.js
var require_retry_operation = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/node_modules/retry/lib/retry_operation.js"(exports, module) {
    function RetryOperation(timeouts, options) {
      if (typeof options === "boolean") {
        options = { forever: options };
      }
      this._originalTimeouts = JSON.parse(JSON.stringify(timeouts));
      this._timeouts = timeouts;
      this._options = options || {};
      this._maxRetryTime = options && options.maxRetryTime || Infinity;
      this._fn = null;
      this._errors = [];
      this._attempts = 1;
      this._operationTimeout = null;
      this._operationTimeoutCb = null;
      this._timeout = null;
      this._operationStart = null;
      if (this._options.forever) {
        this._cachedTimeouts = this._timeouts.slice(0);
      }
    }
    module.exports = RetryOperation;
    RetryOperation.prototype.reset = function() {
      this._attempts = 1;
      this._timeouts = this._originalTimeouts;
    };
    RetryOperation.prototype.stop = function() {
      if (this._timeout) {
        clearTimeout(this._timeout);
      }
      this._timeouts = [];
      this._cachedTimeouts = null;
    };
    RetryOperation.prototype.retry = function(err) {
      if (this._timeout) {
        clearTimeout(this._timeout);
      }
      if (!err) {
        return false;
      }
      var currentTime = (/* @__PURE__ */ new Date()).getTime();
      if (err && currentTime - this._operationStart >= this._maxRetryTime) {
        this._errors.unshift(new Error("RetryOperation timeout occurred"));
        return false;
      }
      this._errors.push(err);
      var timeout = this._timeouts.shift();
      if (timeout === void 0) {
        if (this._cachedTimeouts) {
          this._errors.splice(this._errors.length - 1, this._errors.length);
          this._timeouts = this._cachedTimeouts.slice(0);
          timeout = this._timeouts.shift();
        } else {
          return false;
        }
      }
      var self = this;
      var timer = setTimeout(function() {
        self._attempts++;
        if (self._operationTimeoutCb) {
          self._timeout = setTimeout(function() {
            self._operationTimeoutCb(self._attempts);
          }, self._operationTimeout);
          if (self._options.unref) {
            self._timeout.unref();
          }
        }
        self._fn(self._attempts);
      }, timeout);
      if (this._options.unref) {
        timer.unref();
      }
      return true;
    };
    RetryOperation.prototype.attempt = function(fn, timeoutOps) {
      this._fn = fn;
      if (timeoutOps) {
        if (timeoutOps.timeout) {
          this._operationTimeout = timeoutOps.timeout;
        }
        if (timeoutOps.cb) {
          this._operationTimeoutCb = timeoutOps.cb;
        }
      }
      var self = this;
      if (this._operationTimeoutCb) {
        this._timeout = setTimeout(function() {
          self._operationTimeoutCb();
        }, self._operationTimeout);
      }
      this._operationStart = (/* @__PURE__ */ new Date()).getTime();
      this._fn(this._attempts);
    };
    RetryOperation.prototype.try = function(fn) {
      console.log("Using RetryOperation.try() is deprecated");
      this.attempt(fn);
    };
    RetryOperation.prototype.start = function(fn) {
      console.log("Using RetryOperation.start() is deprecated");
      this.attempt(fn);
    };
    RetryOperation.prototype.start = RetryOperation.prototype.try;
    RetryOperation.prototype.errors = function() {
      return this._errors;
    };
    RetryOperation.prototype.attempts = function() {
      return this._attempts;
    };
    RetryOperation.prototype.mainError = function() {
      if (this._errors.length === 0) {
        return null;
      }
      var counts = {};
      var mainError = null;
      var mainErrorCount = 0;
      for (var i = 0; i < this._errors.length; i++) {
        var error = this._errors[i];
        var message = error.message;
        var count = (counts[message] || 0) + 1;
        counts[message] = count;
        if (count >= mainErrorCount) {
          mainError = error;
          mainErrorCount = count;
        }
      }
      return mainError;
    };
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/node_modules/retry/lib/retry.js
var require_retry = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/node_modules/retry/lib/retry.js"(exports) {
    var RetryOperation = require_retry_operation();
    exports.operation = function(options) {
      var timeouts = exports.timeouts(options);
      return new RetryOperation(timeouts, {
        forever: options && options.forever,
        unref: options && options.unref,
        maxRetryTime: options && options.maxRetryTime
      });
    };
    exports.timeouts = function(options) {
      if (options instanceof Array) {
        return [].concat(options);
      }
      var opts = {
        retries: 10,
        factor: 2,
        minTimeout: 1 * 1e3,
        maxTimeout: Infinity,
        randomize: false
      };
      for (var key in options) {
        opts[key] = options[key];
      }
      if (opts.minTimeout > opts.maxTimeout) {
        throw new Error("minTimeout is greater than maxTimeout");
      }
      var timeouts = [];
      for (var i = 0; i < opts.retries; i++) {
        timeouts.push(this.createTimeout(i, opts));
      }
      if (options && options.forever && !timeouts.length) {
        timeouts.push(this.createTimeout(i, opts));
      }
      timeouts.sort(function(a, b) {
        return a - b;
      });
      return timeouts;
    };
    exports.createTimeout = function(attempt, opts) {
      var random = opts.randomize ? Math.random() + 1 : 1;
      var timeout = Math.round(random * opts.minTimeout * Math.pow(opts.factor, attempt));
      timeout = Math.min(timeout, opts.maxTimeout);
      return timeout;
    };
    exports.wrap = function(obj, options, methods) {
      if (options instanceof Array) {
        methods = options;
        options = null;
      }
      if (!methods) {
        methods = [];
        for (var key in obj) {
          if (typeof obj[key] === "function") {
            methods.push(key);
          }
        }
      }
      for (var i = 0; i < methods.length; i++) {
        var method = methods[i];
        var original = obj[method];
        obj[method] = function retryWrapper(original2) {
          var op = exports.operation(options);
          var args = Array.prototype.slice.call(arguments, 1);
          var callback = args.pop();
          args.push(function(err) {
            if (op.retry(err)) {
              return;
            }
            if (err) {
              arguments[0] = op.mainError();
            }
            callback.apply(this, arguments);
          });
          op.attempt(function() {
            original2.apply(obj, args);
          });
        }.bind(obj, original);
        obj[method].options = options;
      }
    };
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/node_modules/retry/index.js
var require_retry2 = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/node_modules/retry/index.js"(exports, module) {
    module.exports = require_retry();
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/signal-exit/signals.js
var require_signals = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/signal-exit/signals.js"(exports, module) {
    module.exports = [
      "SIGABRT",
      "SIGALRM",
      "SIGHUP",
      "SIGINT",
      "SIGTERM"
    ];
    if (process.platform !== "win32") {
      module.exports.push(
        "SIGVTALRM",
        "SIGXCPU",
        "SIGXFSZ",
        "SIGUSR2",
        "SIGTRAP",
        "SIGSYS",
        "SIGQUIT",
        "SIGIOT"
        // should detect profiler and enable/disable accordingly.
        // see #21
        // 'SIGPROF'
      );
    }
    if (process.platform === "linux") {
      module.exports.push(
        "SIGIO",
        "SIGPOLL",
        "SIGPWR",
        "SIGSTKFLT",
        "SIGUNUSED"
      );
    }
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/signal-exit/index.js
var require_signal_exit = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/signal-exit/index.js"(exports, module) {
    var process2 = global.process;
    var processOk = function(process3) {
      return process3 && typeof process3 === "object" && typeof process3.removeListener === "function" && typeof process3.emit === "function" && typeof process3.reallyExit === "function" && typeof process3.listeners === "function" && typeof process3.kill === "function" && typeof process3.pid === "number" && typeof process3.on === "function";
    };
    if (!processOk(process2)) {
      module.exports = function() {
        return function() {
        };
      };
    } else {
      assert = __require("assert");
      signals = require_signals();
      isWin = /^win/i.test(process2.platform);
      EE = __require("events");
      if (typeof EE !== "function") {
        EE = EE.EventEmitter;
      }
      if (process2.__signal_exit_emitter__) {
        emitter = process2.__signal_exit_emitter__;
      } else {
        emitter = process2.__signal_exit_emitter__ = new EE();
        emitter.count = 0;
        emitter.emitted = {};
      }
      if (!emitter.infinite) {
        emitter.setMaxListeners(Infinity);
        emitter.infinite = true;
      }
      module.exports = function(cb, opts) {
        if (!processOk(global.process)) {
          return function() {
          };
        }
        assert.equal(typeof cb, "function", "a callback must be provided for exit handler");
        if (loaded === false) {
          load();
        }
        var ev = "exit";
        if (opts && opts.alwaysLast) {
          ev = "afterexit";
        }
        var remove = function() {
          emitter.removeListener(ev, cb);
          if (emitter.listeners("exit").length === 0 && emitter.listeners("afterexit").length === 0) {
            unload();
          }
        };
        emitter.on(ev, cb);
        return remove;
      };
      unload = function unload2() {
        if (!loaded || !processOk(global.process)) {
          return;
        }
        loaded = false;
        signals.forEach(function(sig) {
          try {
            process2.removeListener(sig, sigListeners[sig]);
          } catch (er) {
          }
        });
        process2.emit = originalProcessEmit;
        process2.reallyExit = originalProcessReallyExit;
        emitter.count -= 1;
      };
      module.exports.unload = unload;
      emit = function emit2(event, code, signal) {
        if (emitter.emitted[event]) {
          return;
        }
        emitter.emitted[event] = true;
        emitter.emit(event, code, signal);
      };
      sigListeners = {};
      signals.forEach(function(sig) {
        sigListeners[sig] = function listener() {
          if (!processOk(global.process)) {
            return;
          }
          var listeners = process2.listeners(sig);
          if (listeners.length === emitter.count) {
            unload();
            emit("exit", null, sig);
            emit("afterexit", null, sig);
            if (isWin && sig === "SIGHUP") {
              sig = "SIGINT";
            }
            process2.kill(process2.pid, sig);
          }
        };
      });
      module.exports.signals = function() {
        return signals;
      };
      loaded = false;
      load = function load2() {
        if (loaded || !processOk(global.process)) {
          return;
        }
        loaded = true;
        emitter.count += 1;
        signals = signals.filter(function(sig) {
          try {
            process2.on(sig, sigListeners[sig]);
            return true;
          } catch (er) {
            return false;
          }
        });
        process2.emit = processEmit;
        process2.reallyExit = processReallyExit;
      };
      module.exports.load = load;
      originalProcessReallyExit = process2.reallyExit;
      processReallyExit = function processReallyExit2(code) {
        if (!processOk(global.process)) {
          return;
        }
        process2.exitCode = code || /* istanbul ignore next */
        0;
        emit("exit", process2.exitCode, null);
        emit("afterexit", process2.exitCode, null);
        originalProcessReallyExit.call(process2, process2.exitCode);
      };
      originalProcessEmit = process2.emit;
      processEmit = function processEmit2(ev, arg) {
        if (ev === "exit" && processOk(global.process)) {
          if (arg !== void 0) {
            process2.exitCode = arg;
          }
          var ret = originalProcessEmit.apply(this, arguments);
          emit("exit", process2.exitCode, null);
          emit("afterexit", process2.exitCode, null);
          return ret;
        } else {
          return originalProcessEmit.apply(this, arguments);
        }
      };
    }
    var assert;
    var signals;
    var isWin;
    var EE;
    var emitter;
    var unload;
    var emit;
    var sigListeners;
    var loaded;
    var load;
    var originalProcessReallyExit;
    var processReallyExit;
    var originalProcessEmit;
    var processEmit;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/lib/mtime-precision.js
var require_mtime_precision = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/lib/mtime-precision.js"(exports, module) {
    "use strict";
    var cacheSymbol = Symbol();
    function probe(file, fs, callback) {
      const cachedPrecision = fs[cacheSymbol];
      if (cachedPrecision) {
        return fs.stat(file, (err, stat2) => {
          if (err) {
            return callback(err);
          }
          callback(null, stat2.mtime, cachedPrecision);
        });
      }
      const mtime = new Date(Math.ceil(Date.now() / 1e3) * 1e3 + 5);
      fs.utimes(file, mtime, mtime, (err) => {
        if (err) {
          return callback(err);
        }
        fs.stat(file, (err2, stat2) => {
          if (err2) {
            return callback(err2);
          }
          const precision = stat2.mtime.getTime() % 1e3 === 0 ? "s" : "ms";
          Object.defineProperty(fs, cacheSymbol, { value: precision });
          callback(null, stat2.mtime, precision);
        });
      });
    }
    function getMtime(precision) {
      let now = Date.now();
      if (precision === "s") {
        now = Math.ceil(now / 1e3) * 1e3;
      }
      return new Date(now);
    }
    module.exports.probe = probe;
    module.exports.getMtime = getMtime;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/lib/lockfile.js
var require_lockfile = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/lib/lockfile.js"(exports, module) {
    "use strict";
    var path4 = __require("path");
    var fs = require_graceful_fs();
    var retry = require_retry2();
    var onExit = require_signal_exit();
    var mtimePrecision = require_mtime_precision();
    var locks = {};
    function getLockFile(file, options) {
      return options.lockfilePath || `${file}.lock`;
    }
    function resolveCanonicalPath(file, options, callback) {
      if (!options.realpath) {
        return callback(null, path4.resolve(file));
      }
      options.fs.realpath(file, callback);
    }
    function acquireLock(file, options, callback) {
      const lockfilePath = getLockFile(file, options);
      options.fs.mkdir(lockfilePath, (err) => {
        if (!err) {
          return mtimePrecision.probe(lockfilePath, options.fs, (err2, mtime, mtimePrecision2) => {
            if (err2) {
              options.fs.rmdir(lockfilePath, () => {
              });
              return callback(err2);
            }
            callback(null, mtime, mtimePrecision2);
          });
        }
        if (err.code !== "EEXIST") {
          return callback(err);
        }
        if (options.stale <= 0) {
          return callback(Object.assign(new Error("Lock file is already being held"), { code: "ELOCKED", file }));
        }
        options.fs.stat(lockfilePath, (err2, stat2) => {
          if (err2) {
            if (err2.code === "ENOENT") {
              return acquireLock(file, { ...options, stale: 0 }, callback);
            }
            return callback(err2);
          }
          if (!isLockStale(stat2, options)) {
            return callback(Object.assign(new Error("Lock file is already being held"), { code: "ELOCKED", file }));
          }
          removeLock(file, options, (err3) => {
            if (err3) {
              return callback(err3);
            }
            acquireLock(file, { ...options, stale: 0 }, callback);
          });
        });
      });
    }
    function isLockStale(stat2, options) {
      return stat2.mtime.getTime() < Date.now() - options.stale;
    }
    function removeLock(file, options, callback) {
      options.fs.rmdir(getLockFile(file, options), (err) => {
        if (err && err.code !== "ENOENT") {
          return callback(err);
        }
        callback();
      });
    }
    function updateLock(file, options) {
      const lock2 = locks[file];
      if (lock2.updateTimeout) {
        return;
      }
      lock2.updateDelay = lock2.updateDelay || options.update;
      lock2.updateTimeout = setTimeout(() => {
        lock2.updateTimeout = null;
        options.fs.stat(lock2.lockfilePath, (err, stat2) => {
          const isOverThreshold = lock2.lastUpdate + options.stale < Date.now();
          if (err) {
            if (err.code === "ENOENT" || isOverThreshold) {
              return setLockAsCompromised(file, lock2, Object.assign(err, { code: "ECOMPROMISED" }));
            }
            lock2.updateDelay = 1e3;
            return updateLock(file, options);
          }
          const isMtimeOurs = lock2.mtime.getTime() === stat2.mtime.getTime();
          if (!isMtimeOurs) {
            return setLockAsCompromised(
              file,
              lock2,
              Object.assign(
                new Error("Unable to update lock within the stale threshold"),
                { code: "ECOMPROMISED" }
              )
            );
          }
          const mtime = mtimePrecision.getMtime(lock2.mtimePrecision);
          options.fs.utimes(lock2.lockfilePath, mtime, mtime, (err2) => {
            const isOverThreshold2 = lock2.lastUpdate + options.stale < Date.now();
            if (lock2.released) {
              return;
            }
            if (err2) {
              if (err2.code === "ENOENT" || isOverThreshold2) {
                return setLockAsCompromised(file, lock2, Object.assign(err2, { code: "ECOMPROMISED" }));
              }
              lock2.updateDelay = 1e3;
              return updateLock(file, options);
            }
            lock2.mtime = mtime;
            lock2.lastUpdate = Date.now();
            lock2.updateDelay = null;
            updateLock(file, options);
          });
        });
      }, lock2.updateDelay);
      if (lock2.updateTimeout.unref) {
        lock2.updateTimeout.unref();
      }
    }
    function setLockAsCompromised(file, lock2, err) {
      lock2.released = true;
      if (lock2.updateTimeout) {
        clearTimeout(lock2.updateTimeout);
      }
      if (locks[file] === lock2) {
        delete locks[file];
      }
      lock2.options.onCompromised(err);
    }
    function lock(file, options, callback) {
      options = {
        stale: 1e4,
        update: null,
        realpath: true,
        retries: 0,
        fs,
        onCompromised: (err) => {
          throw err;
        },
        ...options
      };
      options.retries = options.retries || 0;
      options.retries = typeof options.retries === "number" ? { retries: options.retries } : options.retries;
      options.stale = Math.max(options.stale || 0, 2e3);
      options.update = options.update == null ? options.stale / 2 : options.update || 0;
      options.update = Math.max(Math.min(options.update, options.stale / 2), 1e3);
      resolveCanonicalPath(file, options, (err, file2) => {
        if (err) {
          return callback(err);
        }
        const operation = retry.operation(options.retries);
        operation.attempt(() => {
          acquireLock(file2, options, (err2, mtime, mtimePrecision2) => {
            if (operation.retry(err2)) {
              return;
            }
            if (err2) {
              return callback(operation.mainError());
            }
            const lock2 = locks[file2] = {
              lockfilePath: getLockFile(file2, options),
              mtime,
              mtimePrecision: mtimePrecision2,
              options,
              lastUpdate: Date.now()
            };
            updateLock(file2, options);
            callback(null, (releasedCallback) => {
              if (lock2.released) {
                return releasedCallback && releasedCallback(Object.assign(new Error("Lock is already released"), { code: "ERELEASED" }));
              }
              unlock(file2, { ...options, realpath: false }, releasedCallback);
            });
          });
        });
      });
    }
    function unlock(file, options, callback) {
      options = {
        fs,
        realpath: true,
        ...options
      };
      resolveCanonicalPath(file, options, (err, file2) => {
        if (err) {
          return callback(err);
        }
        const lock2 = locks[file2];
        if (!lock2) {
          return callback(Object.assign(new Error("Lock is not acquired/owned by you"), { code: "ENOTACQUIRED" }));
        }
        lock2.updateTimeout && clearTimeout(lock2.updateTimeout);
        lock2.released = true;
        delete locks[file2];
        removeLock(file2, options, callback);
      });
    }
    function check(file, options, callback) {
      options = {
        stale: 1e4,
        realpath: true,
        fs,
        ...options
      };
      options.stale = Math.max(options.stale || 0, 2e3);
      resolveCanonicalPath(file, options, (err, file2) => {
        if (err) {
          return callback(err);
        }
        options.fs.stat(getLockFile(file2, options), (err2, stat2) => {
          if (err2) {
            return err2.code === "ENOENT" ? callback(null, false) : callback(err2);
          }
          return callback(null, !isLockStale(stat2, options));
        });
      });
    }
    function getLocks() {
      return locks;
    }
    onExit(() => {
      for (const file in locks) {
        const options = locks[file].options;
        try {
          options.fs.rmdirSync(getLockFile(file, options));
        } catch (e) {
        }
      }
    });
    module.exports.lock = lock;
    module.exports.unlock = unlock;
    module.exports.check = check;
    module.exports.getLocks = getLocks;
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/lib/adapter.js
var require_adapter = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/lib/adapter.js"(exports, module) {
    "use strict";
    var fs = require_graceful_fs();
    function createSyncFs(fs2) {
      const methods = ["mkdir", "realpath", "stat", "rmdir", "utimes"];
      const newFs = { ...fs2 };
      methods.forEach((method) => {
        newFs[method] = (...args) => {
          const callback = args.pop();
          let ret;
          try {
            ret = fs2[`${method}Sync`](...args);
          } catch (err) {
            return callback(err);
          }
          callback(null, ret);
        };
      });
      return newFs;
    }
    function toPromise(method) {
      return (...args) => new Promise((resolve5, reject) => {
        args.push((err, result) => {
          if (err) {
            reject(err);
          } else {
            resolve5(result);
          }
        });
        method(...args);
      });
    }
    function toSync(method) {
      return (...args) => {
        let err;
        let result;
        args.push((_err, _result) => {
          err = _err;
          result = _result;
        });
        method(...args);
        if (err) {
          throw err;
        }
        return result;
      };
    }
    function toSyncOptions(options) {
      options = { ...options };
      options.fs = createSyncFs(options.fs || fs);
      if (typeof options.retries === "number" && options.retries > 0 || options.retries && typeof options.retries.retries === "number" && options.retries.retries > 0) {
        throw Object.assign(new Error("Cannot use retries with the sync api"), { code: "ESYNC" });
      }
      return options;
    }
    module.exports = {
      toPromise,
      toSync,
      toSyncOptions
    };
  }
});

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/index.js
var require_proper_lockfile = __commonJS({
  "../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/proper-lockfile/index.js"(exports, module) {
    "use strict";
    var lockfile4 = require_lockfile();
    var { toPromise, toSync, toSyncOptions } = require_adapter();
    async function lock(file, options) {
      const release = await toPromise(lockfile4.lock)(file, options);
      return toPromise(release);
    }
    function lockSync(file, options) {
      const release = toSync(lockfile4.lock)(file, toSyncOptions(options));
      return toSync(release);
    }
    function unlock(file, options) {
      return toPromise(lockfile4.unlock)(file, options);
    }
    function unlockSync(file, options) {
      return toSync(lockfile4.unlock)(file, toSyncOptions(options));
    }
    function check(file, options) {
      return toPromise(lockfile4.check)(file, options);
    }
    function checkSync(file, options) {
      return toSync(lockfile4.check)(file, toSyncOptions(options));
    }
    module.exports = lock;
    module.exports.lock = lock;
    module.exports.unlock = unlock;
    module.exports.lockSync = lockSync;
    module.exports.unlockSync = unlockSync;
    module.exports.check = check;
    module.exports.checkSync = checkSync;
  }
});

// target/vendor-pi/dist/core/session-manager.js
import { getCurrentSystemMessage, uuidv7 } from "@earendil-works/pi-ai";
import { randomUUID } from "crypto";
import { appendFileSync, closeSync, createReadStream, existsSync, mkdirSync, openSync, readdirSync, readSync, statSync as statSync2, writeFileSync } from "fs";
import { readdir, stat } from "fs/promises";
import { basename, join as join2, resolve } from "path";
import { createInterface } from "readline";
import { StringDecoder } from "string_decoder";

// target/vendor-pi/dist/config.js
var config_exports = {};
__reExport(config_exports, host_star);
import * as host_star from "yapi:pi/host";

// target/vendor-pi/dist/utils/paths.js
import { realpathSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { isAbsolute, join, resolve as nodeResolvePath, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

// target/vendor-pi/dist/utils/child-process.js
var import_cross_spawn = __toESM(require_cross_spawn());
import { spawn as nodeSpawn, spawnSync as nodeSpawnSync } from "node:child_process";
var EXIT_STDIO_GRACE_MS = 100;
function waitForChildProcess(child) {
  return new Promise((resolve5, reject) => {
    let settled = false;
    let exited = false;
    let exitCode = null;
    let postExitTimer;
    let stdoutEnded = child.stdout === null;
    let stderrEnded = child.stderr === null;
    const cleanup = () => {
      if (postExitTimer) {
        clearTimeout(postExitTimer);
        postExitTimer = void 0;
      }
      child.removeListener("error", onError);
      child.removeListener("exit", onExit);
      child.removeListener("close", onClose);
      child.stdout?.removeListener("end", onStdoutEnd);
      child.stderr?.removeListener("end", onStderrEnd);
      child.stdout?.removeListener("data", onData);
      child.stderr?.removeListener("data", onData);
    };
    const finalize = (code) => {
      if (settled)
        return;
      settled = true;
      cleanup();
      child.stdout?.destroy();
      child.stderr?.destroy();
      resolve5(code);
    };
    const maybeFinalizeAfterExit = () => {
      if (!exited || settled)
        return;
      if (stdoutEnded && stderrEnded) {
        finalize(exitCode);
      }
    };
    const armIdleTimer = () => {
      if (postExitTimer)
        clearTimeout(postExitTimer);
      postExitTimer = setTimeout(() => finalize(exitCode), EXIT_STDIO_GRACE_MS);
    };
    const onData = () => {
      if (exited && !settled)
        armIdleTimer();
    };
    const onStdoutEnd = () => {
      stdoutEnded = true;
      maybeFinalizeAfterExit();
    };
    const onStderrEnd = () => {
      stderrEnded = true;
      maybeFinalizeAfterExit();
    };
    const onError = (err) => {
      if (settled)
        return;
      settled = true;
      cleanup();
      reject(err);
    };
    const onExit = (code) => {
      exited = true;
      exitCode = code;
      maybeFinalizeAfterExit();
      if (!settled) {
        armIdleTimer();
      }
    };
    const onClose = (code) => {
      finalize(code);
    };
    child.stdout?.once("end", onStdoutEnd);
    child.stderr?.once("end", onStderrEnd);
    child.stdout?.on("data", onData);
    child.stderr?.on("data", onData);
    child.once("error", onError);
    child.once("exit", onExit);
    child.once("close", onClose);
  });
}

// target/vendor-pi/dist/utils/paths.js
var UNICODE_SPACES = /[\u00A0\u2000-\u200A\u202F\u205F\u3000]/g;
function canonicalizePath(path4) {
  try {
    return realpathSync(path4);
  } catch {
    return path4;
  }
}
function normalizeWindowsShellPath(filePath) {
  if (!filePath.startsWith("/") || filePath.startsWith("//") || filePath.includes("\\"))
    return filePath;
  const match2 = filePath.match(/^\/(?:mnt\/|cygdrive\/)?([a-z])(?:\/(.*))?$/i);
  if (!match2)
    return filePath;
  const suffix = match2[2]?.replaceAll("/", "\\");
  return `${match2[1].toUpperCase()}:\\${suffix ?? ""}`;
}
function normalizePath(input, options = {}) {
  let normalized = options.trim ? input.trim() : input;
  if (options.normalizeUnicodeSpaces) {
    normalized = normalized.replace(UNICODE_SPACES, " ");
  }
  if (options.stripAtPrefix && normalized.startsWith("@")) {
    normalized = normalized.slice(1);
  }
  if (process.platform === "win32") {
    normalized = normalizeWindowsShellPath(normalized);
  }
  if (options.expandTilde ?? true) {
    const home = options.homeDir ?? homedir();
    if (normalized === "~")
      return home;
    if (normalized.startsWith("~/") || process.platform === "win32" && normalized.startsWith("~\\")) {
      return join(home, normalized.slice(2));
    }
  }
  if (/^file:\/\//.test(normalized)) {
    return fileURLToPath(normalized);
  }
  return normalized;
}
function resolvePath(input, baseDir = process.cwd(), options = {}) {
  const normalized = normalizePath(input, options);
  const normalizedBaseDir = normalizePath(baseDir);
  return isAbsolute(normalized) ? nodeResolvePath(normalized) : nodeResolvePath(normalizedBaseDir, normalized);
}
function getCwdRelativePath(filePath, cwd) {
  const resolvedCwd = resolvePath(cwd);
  const resolvedPath = resolvePath(filePath, resolvedCwd);
  const relativePath = relative(resolvedCwd, resolvedPath);
  const isInsideCwd = relativePath === "" || relativePath !== ".." && !relativePath.startsWith(`..${sep}`) && !isAbsolute(relativePath);
  return isInsideCwd ? relativePath || "." : void 0;
}
function formatPathRelativeToCwdOrAbsolute(filePath, cwd) {
  const absolutePath = resolvePath(filePath, cwd);
  return (getCwdRelativePath(absolutePath, cwd) ?? absolutePath).split(sep).join("/");
}

// target/vendor-pi/dist/core/messages.js
var COMPACTION_SUMMARY_PREFIX = `The conversation history before this point was compacted into the following summary:

<summary>
`;
var COMPACTION_SUMMARY_SUFFIX = `
</summary>`;
var BRANCH_SUMMARY_PREFIX = `The following is a summary of a branch that this conversation came back from:

<summary>
`;
var BRANCH_SUMMARY_SUFFIX = `</summary>`;
function bashExecutionToText(msg) {
  let text = `Ran \`${msg.command}\`
`;
  if (msg.output) {
    text += `\`\`\`
${msg.output}
\`\`\``;
  } else {
    text += "(no output)";
  }
  if (msg.cancelled) {
    text += "\n\n(command cancelled)";
  } else if (msg.exitCode !== null && msg.exitCode !== void 0 && msg.exitCode !== 0) {
    text += `

Command exited with code ${msg.exitCode}`;
  }
  if (msg.truncated && msg.fullOutputPath) {
    text += `

[Output truncated. Full output: ${msg.fullOutputPath}]`;
  }
  return text;
}
function createBranchSummaryMessage(summary, fromId, timestamp) {
  return {
    role: "branchSummary",
    summary,
    fromId,
    timestamp: new Date(timestamp).getTime()
  };
}
function createCompactionSummaryMessage(summary, tokensBefore, timestamp) {
  return {
    role: "compactionSummary",
    summary,
    tokensBefore,
    timestamp: new Date(timestamp).getTime()
  };
}
function createCustomMessage(customType, content, display, details, timestamp) {
  return {
    role: "custom",
    customType,
    content,
    display,
    details,
    timestamp: new Date(timestamp).getTime()
  };
}
function convertToLlm(messages) {
  return messages.map((m) => {
    switch (m.role) {
      case "bashExecution":
        if (m.excludeFromContext) {
          return void 0;
        }
        return {
          role: "user",
          content: [{ type: "text", text: bashExecutionToText(m) }],
          timestamp: m.timestamp
        };
      case "custom": {
        const content = typeof m.content === "string" ? [{ type: "text", text: m.content }] : m.content;
        return {
          role: "user",
          content,
          timestamp: m.timestamp
        };
      }
      case "branchSummary":
        return {
          role: "user",
          content: [{ type: "text", text: BRANCH_SUMMARY_PREFIX + m.summary + BRANCH_SUMMARY_SUFFIX }],
          timestamp: m.timestamp
        };
      case "compactionSummary":
        return {
          role: "user",
          content: [
            { type: "text", text: COMPACTION_SUMMARY_PREFIX + m.summary + COMPACTION_SUMMARY_SUFFIX }
          ],
          timestamp: m.timestamp
        };
      case "system":
      case "user":
      case "assistant":
      case "toolResult":
        return m;
      default:
        const _exhaustiveCheck = m;
        return void 0;
    }
  }).filter((m) => m !== void 0);
}

// target/vendor-pi/dist/core/session-manager.js
var CURRENT_SESSION_VERSION = 3;
function createSessionId() {
  return uuidv7();
}
function assertValidSessionId(id) {
  if (!/^[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?$/.test(id)) {
    throw new Error("Session id must be non-empty, contain only alphanumeric characters, '-', '_', and '.', and start and end with an alphanumeric character");
  }
}
function generateId(byId) {
  for (let i = 0; i < 100; i++) {
    const id = randomUUID().slice(0, 8);
    if (!byId.has(id))
      return id;
  }
  return randomUUID();
}
function migrateV1ToV2(entries) {
  const ids = /* @__PURE__ */ new Set();
  let prevId = null;
  for (const entry of entries) {
    if (entry.type === "session") {
      entry.version = 2;
      continue;
    }
    entry.id = generateId(ids);
    entry.parentId = prevId;
    prevId = entry.id;
    if (entry.type === "compaction") {
      const comp = entry;
      if (typeof comp.firstKeptEntryIndex === "number") {
        const targetEntry = entries[comp.firstKeptEntryIndex];
        if (targetEntry && targetEntry.type !== "session") {
          comp.firstKeptEntryId = targetEntry.id;
        }
        delete comp.firstKeptEntryIndex;
      }
    }
  }
}
function migrateV2ToV3(entries) {
  for (const entry of entries) {
    if (entry.type === "session") {
      entry.version = 3;
      continue;
    }
    if (entry.type === "message") {
      const msgEntry = entry;
      if (msgEntry.message && msgEntry.message.role === "hookMessage") {
        msgEntry.message.role = "custom";
      }
    }
  }
}
function migrateToCurrentVersion(entries) {
  const header = entries.find((e) => e.type === "session");
  const version = header?.version ?? 1;
  if (version >= CURRENT_SESSION_VERSION)
    return false;
  if (version < 2)
    migrateV1ToV2(entries);
  if (version < 3)
    migrateV2ToV3(entries);
  return true;
}
function migrateSessionEntries(entries) {
  migrateToCurrentVersion(entries);
}
function parseSessionEntries(content) {
  const entries = [];
  const lines = content.trim().split("\n");
  for (const line of lines) {
    if (!line.trim())
      continue;
    try {
      const entry = JSON.parse(line);
      entries.push(entry);
    } catch {
    }
  }
  return entries;
}
function buildEntryIndex(entries, byId) {
  if (byId)
    return byId;
  const index = /* @__PURE__ */ new Map();
  for (const entry of entries) {
    index.set(entry.id, entry);
  }
  return index;
}
function buildSessionPath(entries, leafId, byId) {
  const index = buildEntryIndex(entries, byId);
  let leaf;
  if (leafId === null) {
    return [];
  }
  if (leafId) {
    leaf = index.get(leafId);
  }
  leaf ??= entries[entries.length - 1];
  if (!leaf) {
    return [];
  }
  const path4 = [];
  let current = leaf;
  while (current) {
    path4.push(current);
    current = current.parentId ? index.get(current.parentId) : void 0;
  }
  path4.reverse();
  return path4;
}
function getSessionContextSettings(path4) {
  let thinkingLevel = "off";
  let model = null;
  for (const entry of path4) {
    if (entry.type === "thinking_level_change") {
      thinkingLevel = entry.thinkingLevel;
    } else if (entry.type === "model_change") {
      model = { provider: entry.provider, modelId: entry.modelId };
    } else if (entry.type === "message" && entry.message.role === "assistant") {
      model = { provider: entry.message.provider, modelId: entry.message.model };
    }
  }
  return { thinkingLevel, model };
}
function sessionEntryToContextMessages(entry) {
  if (entry.type === "message") {
    const message = entry.message;
    if (message.role === "system" && message.content == null)
      return [{ ...message, content: "" }];
    if ((message.role === "user" || message.role === "assistant" || message.role === "toolResult") && message.content == null) {
      return [{ ...message, content: [] }];
    }
    return [message];
  }
  if (entry.type === "custom_message") {
    return [
      createCustomMessage(entry.customType, entry.content ?? [], entry.display, entry.details, entry.timestamp)
    ];
  }
  if (entry.type === "branch_summary" && entry.summary) {
    return [createBranchSummaryMessage(entry.summary, entry.fromId, entry.timestamp)];
  }
  if (entry.type === "compaction") {
    const summary = createCompactionSummaryMessage(entry.summary, entry.tokensBefore, entry.timestamp);
    return entry.systemMessage ? [entry.systemMessage, summary] : [summary];
  }
  return [];
}
function buildContextEntries(entries, leafId, byId) {
  const path4 = buildSessionPath(entries, leafId, byId);
  let compaction = null;
  for (const entry of path4) {
    if (entry.type === "compaction") {
      compaction = entry;
    }
  }
  if (!compaction) {
    return path4;
  }
  const compactionIdx = path4.findIndex((entry) => entry.id === compaction.id);
  if (compactionIdx < 0) {
    return path4;
  }
  const contextEntries = [compaction];
  let foundFirstKept = false;
  for (let i = 0; i < compactionIdx; i++) {
    const entry = path4[i];
    if (entry.id === compaction.firstKeptEntryId) {
      foundFirstKept = true;
    }
    if (foundFirstKept && !(entry.type === "message" && entry.message.role === "system")) {
      contextEntries.push(entry);
    }
  }
  contextEntries.push(...path4.slice(compactionIdx + 1));
  return contextEntries;
}
function projectContextEntry(entry, edit) {
  const messages = sessionEntryToContextMessages(entry);
  if (!edit)
    return messages;
  const replacement = edit.replacement;
  if (replacement === null)
    return [];
  return messages.map((message) => {
    if (message.role !== "user" && message.role !== "assistant" && message.role !== "toolResult" && message.role !== "custom") {
      return message;
    }
    const content = (message.role === "assistant" || message.role === "toolResult") && typeof replacement.content === "string" ? [{ type: "text", text: replacement.content }] : replacement.content;
    return { ...message, content };
  });
}
function buildSessionProjection(entries, leafId, byId) {
  const path4 = buildSessionPath(entries, leafId, byId);
  const { thinkingLevel, model } = getSessionContextSettings(path4);
  const contextEntries = buildContextEntries(entries, leafId, byId);
  const edits = /* @__PURE__ */ new Map();
  for (const entry of contextEntries) {
    if (entry.type === "context_edit")
      edits.set(entry.targetId, entry);
  }
  const projectedEntries = contextEntries.map((sourceEntry, index) => ({
    sourceEntry,
    // buildContextEntries() may retain an older compaction entry because its
    // raw ID lies inside the newest retained range. Only the newest compaction
    // at index zero contributes a checkpoint and summary.
    messages: sourceEntry.type === "compaction" && index > 0 ? [] : projectContextEntry(sourceEntry, edits.get(sourceEntry.id))
  }));
  return {
    entries: projectedEntries,
    messages: projectedEntries.flatMap((entry) => entry.messages),
    thinkingLevel,
    model
  };
}
function buildSessionContext(entries, leafId, byId) {
  const { messages, thinkingLevel, model } = buildSessionProjection(entries, leafId, byId);
  return { messages, thinkingLevel, model };
}
function getDefaultSessionDirPath(cwd, agentDir = (0, config_exports.getAgentDir)()) {
  const resolvedCwd = resolvePath(cwd);
  const resolvedAgentDir = resolvePath(agentDir);
  const safePath = `--${resolvedCwd.replace(/^[/\\]/, "").replace(/[/\\:]/g, "-")}--`;
  return join2(resolvedAgentDir, "sessions", safePath);
}
function getDefaultSessionDir(cwd, agentDir = (0, config_exports.getAgentDir)()) {
  const sessionDir = getDefaultSessionDirPath(cwd, agentDir);
  if (!existsSync(sessionDir)) {
    mkdirSync(sessionDir, { recursive: true });
  }
  return sessionDir;
}
var SESSION_READ_BUFFER_SIZE = 1024 * 1024;
var SESSION_HEADER_READ_BUFFER_SIZE = 4096;
var MAX_SESSION_HEADER_SCAN_BYTES = 1024 * 1024;
var SessionHeaderScanLimitError = class extends Error {
  constructor(filePath) {
    super(`Session header exceeds ${MAX_SESSION_HEADER_SCAN_BYTES}-byte scan limit: ${filePath}`);
    this.name = "SessionHeaderScanLimitError";
  }
};
function parseSessionEntryLine(line) {
  if (!line.trim())
    return null;
  try {
    return JSON.parse(line);
  } catch {
    return null;
  }
}
function loadEntriesFromFile(filePath) {
  const resolvedFilePath = normalizePath(filePath);
  if (!existsSync(resolvedFilePath))
    return [];
  const entries = [];
  let pending = "";
  const fd = openSync(resolvedFilePath, "r");
  try {
    const decoder = new StringDecoder("utf8");
    const buffer = Buffer.allocUnsafe(SESSION_READ_BUFFER_SIZE);
    while (true) {
      const bytesRead = readSync(fd, buffer, 0, buffer.length, null);
      if (bytesRead === 0)
        break;
      pending += decoder.write(buffer.subarray(0, bytesRead));
      let lineStart = 0;
      let newlineIndex = pending.indexOf("\n", lineStart);
      while (newlineIndex !== -1) {
        const entry = parseSessionEntryLine(pending.slice(lineStart, newlineIndex));
        if (entry)
          entries.push(entry);
        lineStart = newlineIndex + 1;
        newlineIndex = pending.indexOf("\n", lineStart);
      }
      pending = pending.slice(lineStart);
    }
    pending += decoder.end();
    const finalEntry = parseSessionEntryLine(pending);
    if (finalEntry)
      entries.push(finalEntry);
  } finally {
    closeSync(fd);
  }
  if (entries.length === 0)
    return entries;
  const header = entries[0];
  if (header.type !== "session" || typeof header.id !== "string") {
    return [];
  }
  if (pending)
    appendFileSync(resolvedFilePath, "\n");
  return entries;
}
function parseSessionHeaderCandidate(line) {
  if (!line.trim())
    return void 0;
  const entry = parseSessionEntryLine(line);
  if (!entry)
    return void 0;
  if (entry.type !== "session" || typeof entry.id !== "string")
    return null;
  return entry;
}
function readSessionHeader(filePath) {
  const fd = openSync(filePath, "r");
  try {
    const decoder = new StringDecoder("utf8");
    const buffer = Buffer.allocUnsafe(SESSION_HEADER_READ_BUFFER_SIZE);
    const lineChunks = [];
    let scannedBytes = 0;
    while (scannedBytes < MAX_SESSION_HEADER_SCAN_BYTES) {
      const readLength = Math.min(buffer.length, MAX_SESSION_HEADER_SCAN_BYTES - scannedBytes);
      const bytesRead = readSync(fd, buffer, 0, readLength, null);
      if (bytesRead === 0) {
        lineChunks.push(decoder.end());
        return parseSessionHeaderCandidate(lineChunks.join("")) ?? null;
      }
      scannedBytes += bytesRead;
      const chunk = decoder.write(buffer.subarray(0, bytesRead));
      let lineStart = 0;
      let newlineIndex = chunk.indexOf("\n", lineStart);
      while (newlineIndex !== -1) {
        lineChunks.push(chunk.slice(lineStart, newlineIndex));
        const header = parseSessionHeaderCandidate(lineChunks.join(""));
        if (header !== void 0)
          return header;
        lineChunks.length = 0;
        lineStart = newlineIndex + 1;
        newlineIndex = chunk.indexOf("\n", lineStart);
      }
      lineChunks.push(chunk.slice(lineStart));
    }
    const probe = Buffer.allocUnsafe(1);
    if (readSync(fd, probe, 0, probe.length, null) === 0) {
      lineChunks.push(decoder.end());
      return parseSessionHeaderCandidate(lineChunks.join("")) ?? null;
    }
    throw new SessionHeaderScanLimitError(filePath);
  } finally {
    closeSync(fd);
  }
}
function readSessionHeaderForDiscovery(filePath) {
  try {
    return readSessionHeader(filePath);
  } catch {
    return null;
  }
}
function getSessionHeaderCwd(header) {
  const cwd = header.cwd;
  return typeof cwd === "string" ? cwd : void 0;
}
function sessionCwdMatches(cwd, resolvedCwd) {
  return cwd !== void 0 && cwd !== "" && resolvePath(cwd) === resolvedCwd;
}
function findMostRecentSession(sessionDir, cwd) {
  const resolvedSessionDir = normalizePath(sessionDir);
  const resolvedCwd = cwd ? resolvePath(cwd) : void 0;
  try {
    const files = readdirSync(resolvedSessionDir).filter((file) => file.endsWith(".jsonl")).map((file) => join2(resolvedSessionDir, file)).map((path4) => ({ path: path4, mtime: statSync2(path4).mtimeMs })).sort((a, b) => b.mtime - a.mtime);
    for (const { path: path4 } of files) {
      const header = readSessionHeaderForDiscovery(path4);
      if (header && (!resolvedCwd || sessionCwdMatches(getSessionHeaderCwd(header), resolvedCwd)))
        return path4;
    }
    return null;
  } catch {
    return null;
  }
}
function isMessageWithContent(message) {
  return typeof message.role === "string" && "content" in message;
}
function extractTextContent(message) {
  const content = message.content;
  if (typeof content === "string") {
    return content;
  }
  return content.filter((block) => block.type === "text").map((block) => block.text).join(" ");
}
function getMessageActivityTime(entry) {
  const message = entry.message;
  if (!isMessageWithContent(message))
    return void 0;
  if (message.role !== "user" && message.role !== "assistant")
    return void 0;
  const msgTimestamp = message.timestamp;
  if (typeof msgTimestamp === "number") {
    return msgTimestamp;
  }
  const t = new Date(entry.timestamp).getTime();
  return Number.isNaN(t) ? void 0 : t;
}
async function buildSessionInfo(filePath, signal, fileStats) {
  try {
    const stats = fileStats ?? await stat(filePath);
    let header = null;
    let messageCount = 0;
    let firstMessage = "";
    const allMessages = [];
    let name;
    let lastActivityTime;
    const rl = createInterface({
      input: createReadStream(filePath, { encoding: "utf8", signal }),
      crlfDelay: Infinity
    });
    for await (const line of rl) {
      const entry = parseSessionEntryLine(line);
      if (!entry)
        continue;
      if (!header) {
        if (entry.type !== "session")
          return null;
        header = entry;
        continue;
      }
      if (entry.type === "session_info") {
        name = entry.name?.trim() || void 0;
      }
      if (entry.type !== "message")
        continue;
      messageCount++;
      const activityTime = getMessageActivityTime(entry);
      if (typeof activityTime === "number") {
        lastActivityTime = Math.max(lastActivityTime ?? 0, activityTime);
      }
      const message = entry.message;
      if (!isMessageWithContent(message))
        continue;
      if (message.role !== "user" && message.role !== "assistant")
        continue;
      const textContent = extractTextContent(message);
      if (!textContent)
        continue;
      allMessages.push(textContent);
      if (!firstMessage && message.role === "user") {
        firstMessage = textContent;
      }
    }
    if (!header)
      return null;
    const cwd = typeof header.cwd === "string" ? header.cwd : "";
    const parentSessionPath = header.parentSession;
    const headerTime = typeof header.timestamp === "string" ? new Date(header.timestamp).getTime() : NaN;
    const modified = typeof lastActivityTime === "number" && lastActivityTime > 0 ? new Date(lastActivityTime) : !Number.isNaN(headerTime) ? new Date(headerTime) : stats.mtime;
    return {
      path: filePath,
      id: header.id,
      cwd,
      name,
      parentSessionPath,
      created: new Date(header.timestamp),
      modified,
      messageCount,
      firstMessage: firstMessage || "(no messages)",
      allMessagesText: allMessages.join(" ")
    };
  } catch {
    signal?.throwIfAborted();
    return null;
  }
}
var MAX_CONCURRENT_SESSION_INFO_LOADS = 10;
var MAX_CONCURRENT_SESSION_DISCOVERY_LOADS = 64;
var CURRENT_SESSION_LIST_PUBLISH_INTERVAL = 10;
var ALL_SESSION_LIST_PUBLISH_INTERVAL = 100;
async function mapWithConcurrency(items, limit, map, signal) {
  const results = new Array(items.length);
  let nextIndex = 0;
  const worker = async () => {
    while (nextIndex < items.length) {
      signal?.throwIfAborted();
      const index = nextIndex++;
      results[index] = await map(items[index], index);
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, () => worker()));
  return results;
}
function sortSessionInfos(sessions) {
  return sessions.sort((a, b) => b.modified.getTime() - a.modified.getTime());
}
function buildSessionInfosWithConcurrency(files, onLoaded, signal) {
  return mapWithConcurrency(files, MAX_CONCURRENT_SESSION_INFO_LOADS, async (file, index) => {
    const info = await buildSessionInfo(file.path, signal, file.stats);
    onLoaded(info, index);
    return info;
  }, signal);
}
async function listSessionsFromDir(dir, onProgress, signal) {
  signal?.throwIfAborted();
  if (!existsSync(dir))
    return [];
  try {
    const dirEntries = await readdir(dir);
    const files = dirEntries.filter((file) => file.endsWith(".jsonl")).sort((a, b) => b.localeCompare(a)).map((file) => ({ path: join2(dir, file) }));
    const total = files.length;
    const partialSessions = [];
    let loaded = 0;
    const results = await buildSessionInfosWithConcurrency(files, (info) => {
      loaded++;
      if (info)
        partialSessions.push(info);
      const publishPartial = loaded === 1 || loaded % CURRENT_SESSION_LIST_PUBLISH_INTERVAL === 0 || loaded === files.length;
      onProgress?.(loaded, total, publishPartial ? sortSessionInfos([...partialSessions]) : void 0);
    }, signal);
    return results.filter((info) => info !== null);
  } catch {
    signal?.throwIfAborted();
    return [];
  }
}
var SessionManager = class _SessionManager {
  sessionId = "";
  sessionFile;
  sessionDir;
  cwd;
  persist;
  flushed = false;
  fileEntries = [];
  byId = /* @__PURE__ */ new Map();
  labelsById = /* @__PURE__ */ new Map();
  labelTimestampsById = /* @__PURE__ */ new Map();
  leafId = null;
  constructor(cwd, sessionDir, sessionFile, persist, newSessionOptions, preloadedFileEntries) {
    this.cwd = resolvePath(cwd);
    this.sessionDir = normalizePath(sessionDir);
    this.persist = persist;
    if (persist && this.sessionDir && !existsSync(this.sessionDir)) {
      mkdirSync(this.sessionDir, { recursive: true });
    }
    if (sessionFile) {
      this._setSessionFile(sessionFile, preloadedFileEntries);
    } else if (preloadedFileEntries?.length) {
      this._loadEntries(preloadedFileEntries, newSessionOptions);
    } else {
      this.newSession(newSessionOptions);
    }
  }
  /** Switch to a different session file (used for resume and branching) */
  setSessionFile(sessionFile) {
    this._setSessionFile(sessionFile);
  }
  _setSessionFile(sessionFile, preloadedFileEntries) {
    this.sessionFile = resolvePath(sessionFile);
    if (existsSync(this.sessionFile)) {
      const entries = preloadedFileEntries ?? loadEntriesFromFile(this.sessionFile);
      if (entries.length === 0) {
        const explicitPath = this.sessionFile;
        if (statSync2(explicitPath).size > 0) {
          throw new Error(`Session file is not a valid ${config_exports.APP_NAME} session: ${explicitPath}`);
        }
        this.newSession();
        this.sessionFile = explicitPath;
        this._rewriteFile();
        this.flushed = true;
        return;
      }
      this._loadEntries(entries);
      this.flushed = true;
    } else {
      const explicitPath = this.sessionFile;
      this.newSession();
      this.sessionFile = explicitPath;
    }
  }
  newSession(options) {
    if (options?.id !== void 0) {
      assertValidSessionId(options.id);
    }
    this.sessionId = options?.id ?? createSessionId();
    const timestamp = (/* @__PURE__ */ new Date()).toISOString();
    const header = {
      type: "session",
      version: CURRENT_SESSION_VERSION,
      id: this.sessionId,
      timestamp,
      cwd: this.cwd,
      parentSession: options?.parentSession
    };
    this.fileEntries = [header];
    this.byId.clear();
    this.labelsById.clear();
    this.labelTimestampsById.clear();
    this.leafId = null;
    this.flushed = false;
    if (this.persist) {
      const fileTimestamp = timestamp.replace(/[:.]/g, "-");
      this.sessionFile = join2(this.getSessionDir(), `${fileTimestamp}_${this.sessionId}.jsonl`);
    }
    return this.sessionFile;
  }
  _loadEntries(entries, options) {
    const header = entries.find((e) => e.type === "session");
    if (header) {
      this.fileEntries = entries;
      this.sessionId = header.id;
      if (migrateToCurrentVersion(this.fileEntries)) {
        this._rewriteFile();
      }
    } else {
      this.newSession(options);
      this.fileEntries = this.fileEntries.concat(entries);
    }
    this._buildIndex();
  }
  _buildIndex() {
    this.byId.clear();
    this.labelsById.clear();
    this.labelTimestampsById.clear();
    this.leafId = null;
    for (const entry of this.fileEntries) {
      if (entry.type === "session")
        continue;
      this.byId.set(entry.id, entry);
      this.leafId = entry.id;
      if (entry.type === "label") {
        if (entry.label) {
          this.labelsById.set(entry.targetId, entry.label);
          this.labelTimestampsById.set(entry.targetId, entry.timestamp);
        } else {
          this.labelsById.delete(entry.targetId);
          this.labelTimestampsById.delete(entry.targetId);
        }
      }
    }
  }
  _rewriteFile() {
    if (!this.persist || !this.sessionFile)
      return;
    const fd = openSync(this.sessionFile, "w");
    try {
      for (const entry of this.fileEntries) {
        writeFileSync(fd, `${JSON.stringify(entry)}
`);
      }
    } finally {
      closeSync(fd);
    }
  }
  isPersisted() {
    return this.persist;
  }
  getCwd() {
    return this.cwd;
  }
  getSessionDir() {
    return this.sessionDir;
  }
  usesDefaultSessionDir() {
    return this.sessionDir === getDefaultSessionDirPath(this.cwd);
  }
  getSessionId() {
    return this.sessionId;
  }
  getSessionFile() {
    return this.sessionFile;
  }
  /**
   * A new session file is created only once the session contains a user or assistant message.
   * Setup entries alone (model, thinking level, system prompt) stay in memory so opening and
   * closing pi without chatting leaves no file behind. Starting at the user message (not the
   * first assistant reply) keeps the prompt on disk if the first turn never completes (#10000).
   */
  _hasConversation() {
    return this.fileEntries.some((e) => e.type === "message" && (e.message.role === "user" || e.message.role === "assistant"));
  }
  _persist(entry) {
    if (!this.persist || !this.sessionFile)
      return;
    if (!this.flushed) {
      if (!this._hasConversation())
        return;
      const fd = openSync(this.sessionFile, "wx");
      try {
        for (const e of this.fileEntries) {
          writeFileSync(fd, `${JSON.stringify(e)}
`);
        }
      } finally {
        closeSync(fd);
      }
      this.flushed = true;
    } else {
      appendFileSync(this.sessionFile, `${JSON.stringify(entry)}
`);
    }
  }
  _appendEntry(entry) {
    this.fileEntries.push(entry);
    this.byId.set(entry.id, entry);
    this.leafId = entry.id;
    this._persist(entry);
  }
  /** Append a message as child of current leaf, then advance leaf. Returns entry id.
   * Does not allow writing CompactionSummaryMessage and BranchSummaryMessage directly.
   * Reason: we want these to be top-level entries in the session, not message session entries,
   * so it is easier to find them.
   * These need to be appended via appendCompaction() and appendBranchSummary() methods.
   */
  appendMessage(message) {
    const entry = {
      type: "message",
      id: generateId(this.byId),
      parentId: this.leafId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString(),
      message
    };
    this._appendEntry(entry);
    return entry.id;
  }
  /** Append a thinking level change as child of current leaf, then advance leaf. Returns entry id. */
  appendThinkingLevelChange(thinkingLevel) {
    const entry = {
      type: "thinking_level_change",
      id: generateId(this.byId),
      parentId: this.leafId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString(),
      thinkingLevel
    };
    this._appendEntry(entry);
    return entry.id;
  }
  /** Append a model change as child of current leaf, then advance leaf. Returns entry id. */
  appendModelChange(provider, modelId) {
    const entry = {
      type: "model_change",
      id: generateId(this.byId),
      parentId: this.leafId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString(),
      provider,
      modelId
    };
    this._appendEntry(entry);
    return entry.id;
  }
  /** Append model-attributed usage that does not participate in LLM context. Returns the appended entry. */
  appendUsage(kind, provider, model, usage, note) {
    const entry = {
      type: "usage",
      id: generateId(this.byId),
      parentId: this.leafId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString(),
      kind,
      provider,
      model,
      usage,
      ...note ? { note } : {}
    };
    this._appendEntry(entry);
    return entry;
  }
  /** Append a compaction summary as child of current leaf, then advance leaf. Returns entry id. */
  appendCompaction(summary, firstKeptEntryId, tokensBefore, details, fromHook, usage) {
    const timestamp = (/* @__PURE__ */ new Date()).toISOString();
    const systemMessage = getCurrentSystemMessage(this.buildSessionProjection().messages);
    const id = generateId(this.byId);
    const entry = {
      type: "compaction",
      id,
      parentId: this.leafId,
      timestamp,
      summary,
      firstKeptEntryId: firstKeptEntryId ?? id,
      tokensBefore,
      details,
      usage,
      fromHook,
      ...systemMessage ? { systemMessage: { ...systemMessage, timestamp: new Date(timestamp).getTime() } } : {}
    };
    this._appendEntry(entry);
    return entry.id;
  }
  /** Append a custom entry (for extensions) as child of current leaf, then advance leaf. Returns entry id. */
  appendCustomEntry(customType, data) {
    const entry = {
      type: "custom",
      customType,
      data,
      id: generateId(this.byId),
      parentId: this.leafId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString()
    };
    this._appendEntry(entry);
    return entry.id;
  }
  /** Append a session info entry (e.g., display name). Returns entry id. */
  appendSessionInfo(name) {
    const sanitizedName = name.replace(/[\r\n]+/g, " ").trim();
    const entry = {
      type: "session_info",
      id: generateId(this.byId),
      parentId: this.leafId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString(),
      name: sanitizedName
    };
    this._appendEntry(entry);
    return entry.id;
  }
  /** Get the current session name from the latest session_info entry, if any. */
  getSessionName() {
    for (let i = this.fileEntries.length - 1; i >= 0; i--) {
      const entry = this.fileEntries[i];
      if (entry.type === "session_info") {
        return entry.name?.trim() || void 0;
      }
    }
    return void 0;
  }
  /**
   * Append a custom message entry (for extensions) that participates in LLM context.
   * @param customType Extension identifier for filtering on reload
   * @param content Message content (string or TextContent/ImageContent array)
   * @param display Whether to show in TUI (true = styled display, false = hidden)
   * @param details Optional extension-specific metadata (not sent to LLM)
   * @returns Entry id
   */
  appendCustomMessageEntry(customType, content, display, details) {
    const entry = {
      type: "custom_message",
      customType,
      content,
      display,
      details,
      id: generateId(this.byId),
      parentId: this.leafId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString()
    };
    this._appendEntry(entry);
    return entry.id;
  }
  /** Append a branch-local edit to an earlier model-visible entry. */
  appendContextEdit(targetId, replacement) {
    if (replacement !== null && (typeof replacement !== "object" || !("content" in replacement) || typeof replacement.content !== "string" && !Array.isArray(replacement.content))) {
      throw new Error("Context edit replacement must be null or contain string/array content");
    }
    const target = this.byId.get(targetId);
    if (!target)
      throw new Error(`Entry ${targetId} not found`);
    if (!this.getBranch().some((entry2) => entry2.id === targetId)) {
      throw new Error(`Entry ${targetId} is not on the active branch`);
    }
    const editable = target.type === "custom_message" || target.type === "message" && (target.message.role === "user" || target.message.role === "assistant" || target.message.role === "toolResult");
    if (!editable)
      throw new Error(`Entry ${targetId} does not contribute editable model content`);
    const targetRole = target.type === "message" ? target.message.role : "custom";
    const normalizedReplacement = replacement !== null && (targetRole === "assistant" || targetRole === "toolResult") && typeof replacement.content === "string" ? { content: [{ type: "text", text: replacement.content }] } : replacement;
    const entry = {
      type: "context_edit",
      id: generateId(this.byId),
      parentId: this.leafId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString(),
      targetId,
      replacement: normalizedReplacement
    };
    this._appendEntry(entry);
    return entry.id;
  }
  // =========================================================================
  // Tree Traversal
  // =========================================================================
  getLeafId() {
    return this.leafId;
  }
  getLeafEntry() {
    return this.leafId ? this.byId.get(this.leafId) : void 0;
  }
  getEntry(id) {
    return this.byId.get(id);
  }
  /**
   * Get all direct children of an entry.
   */
  getChildren(parentId) {
    const children = [];
    for (const entry of this.byId.values()) {
      if (entry.parentId === parentId) {
        children.push(entry);
      }
    }
    return children;
  }
  /**
   * Get the label for an entry, if any.
   */
  getLabel(id) {
    return this.labelsById.get(id);
  }
  /**
   * Set or clear a label on an entry.
   * Labels are user-defined markers for bookmarking/navigation.
   * Pass undefined or empty string to clear the label.
   */
  appendLabelChange(targetId, label) {
    if (!this.byId.has(targetId)) {
      throw new Error(`Entry ${targetId} not found`);
    }
    const entry = {
      type: "label",
      id: generateId(this.byId),
      parentId: this.leafId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString(),
      targetId,
      label
    };
    this._appendEntry(entry);
    if (label) {
      this.labelsById.set(targetId, label);
      this.labelTimestampsById.set(targetId, entry.timestamp);
    } else {
      this.labelsById.delete(targetId);
      this.labelTimestampsById.delete(targetId);
    }
    return entry.id;
  }
  /**
   * Walk from entry to root, returning all entries in path order.
   * Includes all entry types (messages, compaction, model changes, etc.).
   * Use buildSessionContext() to get the resolved messages for the LLM.
   */
  getBranch(fromId) {
    const path4 = [];
    const startId = fromId ?? this.leafId;
    let current = startId ? this.byId.get(startId) : void 0;
    while (current) {
      path4.push(current);
      current = current.parentId ? this.byId.get(current.parentId) : void 0;
    }
    path4.reverse();
    return path4;
  }
  /**
   * Build the active, compaction-aware entry list for context/rendering.
   * Uses tree traversal from current leaf.
   */
  buildContextEntries() {
    return buildContextEntries(this.getEntries(), this.leafId, this.byId);
  }
  /**
   * Build the session context (what gets sent to the LLM).
   * Uses tree traversal from current leaf.
   */
  buildSessionProjection() {
    return buildSessionProjection(this.getEntries(), this.leafId, this.byId);
  }
  buildSessionContext() {
    const { messages, thinkingLevel, model } = this.buildSessionProjection();
    return { messages, thinkingLevel, model };
  }
  /**
   * Get session header.
   */
  getHeader() {
    const h = this.fileEntries.find((e) => e.type === "session");
    return h ? h : null;
  }
  /** Number of session entries (excludes header), without copying them like `getEntries()`. */
  getEntryCount() {
    return this.byId.size;
  }
  /**
   * Get all session entries (excludes header). Returns a shallow copy.
   * The session is append-only: use appendXXX() to add entries, branch() to
   * change the leaf pointer. Entries cannot be modified or deleted.
   */
  getEntries() {
    return this.fileEntries.filter((e) => e.type !== "session");
  }
  /**
   * Get the session as a tree structure. Returns a shallow defensive copy of all entries.
   * A well-formed session has exactly one root (first entry with parentId === null).
   * Orphaned entries (broken parent chain) are also returned as roots.
   */
  getTree() {
    const entries = this.getEntries();
    const nodeMap = /* @__PURE__ */ new Map();
    const roots = [];
    for (const entry of entries) {
      const label = this.labelsById.get(entry.id);
      const labelTimestamp = this.labelTimestampsById.get(entry.id);
      nodeMap.set(entry.id, { entry, children: [], label, labelTimestamp });
    }
    for (const entry of entries) {
      const node = nodeMap.get(entry.id);
      if (entry.parentId === null || entry.parentId === entry.id) {
        roots.push(node);
      } else {
        const parent = nodeMap.get(entry.parentId);
        if (parent) {
          parent.children.push(node);
        } else {
          roots.push(node);
        }
      }
    }
    const stack = [...roots];
    while (stack.length > 0) {
      const node = stack.pop();
      node.children.sort((a, b) => new Date(a.entry.timestamp).getTime() - new Date(b.entry.timestamp).getTime());
      stack.push(...node.children);
    }
    return roots;
  }
  // =========================================================================
  // Branching
  // =========================================================================
  /**
   * Start a new branch from an earlier entry.
   * Moves the leaf pointer to the specified entry. The next appendXXX() call
   * will create a child of that entry, forming a new branch. Existing entries
   * are not modified or deleted.
   */
  branch(branchFromId) {
    if (!this.byId.has(branchFromId)) {
      throw new Error(`Entry ${branchFromId} not found`);
    }
    this.leafId = branchFromId;
  }
  /**
   * Reset the leaf pointer to null (before any entries).
   * The next appendXXX() call will create a new root entry (parentId = null).
   * Use this when navigating to re-edit the first user message.
   */
  resetLeaf() {
    this.leafId = null;
  }
  /**
   * Start a new branch with a summary of the abandoned path.
   * Same as branch(), but also appends a branch_summary entry that captures
   * context from the abandoned conversation path.
   */
  branchWithSummary(branchFromId, summary, details, fromHook, usage) {
    if (branchFromId !== null && !this.byId.has(branchFromId)) {
      throw new Error(`Entry ${branchFromId} not found`);
    }
    const fromId = this.leafId ?? "root";
    this.leafId = branchFromId;
    const entry = {
      type: "branch_summary",
      id: generateId(this.byId),
      parentId: branchFromId,
      timestamp: (/* @__PURE__ */ new Date()).toISOString(),
      fromId,
      summary,
      details,
      usage,
      fromHook
    };
    this._appendEntry(entry);
    return entry.id;
  }
  /**
   * Create a new session file containing only the path from root to the specified leaf.
   * Useful for extracting a single conversation path from a branched session.
   * Returns the new session file path, or undefined if not persisting.
   */
  createBranchedSession(leafId) {
    const previousSessionFile = this.sessionFile;
    const path4 = this.getBranch(leafId);
    if (path4.length === 0) {
      throw new Error(`Entry ${leafId} not found`);
    }
    const pathWithoutLabels = [];
    const replacementByLabelId = /* @__PURE__ */ new Map();
    const pendingLabelIds = [];
    let pathParentId = null;
    for (const entry of path4) {
      if (entry.type === "label") {
        pendingLabelIds.push(entry.id);
        continue;
      }
      for (const labelId of pendingLabelIds) {
        replacementByLabelId.set(labelId, entry.id);
      }
      pendingLabelIds.length = 0;
      pathWithoutLabels.push(entry.type === "compaction" ? {
        ...entry,
        parentId: pathParentId,
        firstKeptEntryId: entry.firstKeptEntryId === entry.id ? entry.id : replacementByLabelId.get(entry.firstKeptEntryId) ?? entry.firstKeptEntryId
      } : { ...entry, parentId: pathParentId });
      pathParentId = entry.id;
    }
    const newSessionId = createSessionId();
    const timestamp = (/* @__PURE__ */ new Date()).toISOString();
    const fileTimestamp = timestamp.replace(/[:.]/g, "-");
    const newSessionFile = join2(this.getSessionDir(), `${fileTimestamp}_${newSessionId}.jsonl`);
    const header = {
      type: "session",
      version: CURRENT_SESSION_VERSION,
      id: newSessionId,
      timestamp,
      cwd: this.cwd,
      parentSession: this.persist ? previousSessionFile : void 0
    };
    const pathEntryIds = new Set(pathWithoutLabels.map((e) => e.id));
    const labelsToWrite = [];
    for (const [targetId, label] of this.labelsById) {
      if (pathEntryIds.has(targetId)) {
        labelsToWrite.push({ targetId, label, timestamp: this.labelTimestampsById.get(targetId) });
      }
    }
    if (this.persist) {
      const lastEntryId = pathWithoutLabels[pathWithoutLabels.length - 1]?.id || null;
      let parentId2 = lastEntryId;
      const labelEntries2 = [];
      for (const { targetId, label, timestamp: labelTimestamp } of labelsToWrite) {
        const labelEntry = {
          type: "label",
          id: generateId(new Set(pathEntryIds)),
          parentId: parentId2,
          timestamp: labelTimestamp,
          targetId,
          label
        };
        pathEntryIds.add(labelEntry.id);
        labelEntries2.push(labelEntry);
        parentId2 = labelEntry.id;
      }
      this.fileEntries = [header, ...pathWithoutLabels, ...labelEntries2];
      this.sessionId = newSessionId;
      this.sessionFile = newSessionFile;
      this._buildIndex();
      if (this._hasConversation()) {
        this._rewriteFile();
        this.flushed = true;
      } else {
        this.flushed = false;
      }
      return newSessionFile;
    }
    const labelEntries = [];
    let parentId = pathWithoutLabels[pathWithoutLabels.length - 1]?.id || null;
    for (const { targetId, label, timestamp: labelTimestamp } of labelsToWrite) {
      const labelEntry = {
        type: "label",
        id: generateId(/* @__PURE__ */ new Set([...pathEntryIds, ...labelEntries.map((e) => e.id)])),
        parentId,
        timestamp: labelTimestamp,
        targetId,
        label
      };
      labelEntries.push(labelEntry);
      parentId = labelEntry.id;
    }
    this.fileEntries = [header, ...pathWithoutLabels, ...labelEntries];
    this.sessionId = newSessionId;
    this._buildIndex();
    return void 0;
  }
  /**
   * Create a new session.
   * @param cwd Working directory (stored in session header)
   * @param sessionDir Optional session directory. If omitted, uses default (~/.pi/agent/sessions/<encoded-cwd>/).
   */
  static create(cwd, sessionDir, options) {
    const dir = sessionDir ? normalizePath(sessionDir) : getDefaultSessionDir(cwd);
    return new _SessionManager(cwd, dir, void 0, true, options);
  }
  /**
   * Open a specific session file.
   * @param path Path to session file
   * @param sessionDir Optional session directory for /new or /branch. If omitted, derives from file's parent.
   * @param cwdOverride Optional cwd override instead of the session header cwd.
   */
  static open(path4, sessionDir, cwdOverride) {
    const resolvedPath = resolvePath(path4);
    let header = null;
    let preloadedFileEntries;
    if (cwdOverride === void 0 && existsSync(resolvedPath)) {
      try {
        header = readSessionHeader(resolvedPath);
      } catch (error) {
        if (!(error instanceof SessionHeaderScanLimitError))
          throw error;
        preloadedFileEntries = loadEntriesFromFile(resolvedPath);
        const firstEntry = preloadedFileEntries[0];
        header = firstEntry?.type === "session" ? firstEntry : null;
      }
    }
    const cwd = cwdOverride ?? (header ? getSessionHeaderCwd(header) : void 0) ?? process.cwd();
    const dir = sessionDir ? normalizePath(sessionDir) : resolve(resolvedPath, "..");
    return new _SessionManager(cwd, dir, resolvedPath, true, void 0, preloadedFileEntries);
  }
  /**
   * Continue the most recent session, or create new if none.
   * @param cwd Working directory
   * @param sessionDir Optional session directory. If omitted, uses default (~/.pi/agent/sessions/<encoded-cwd>/).
   */
  static continueRecent(cwd, sessionDir) {
    const dir = sessionDir ? normalizePath(sessionDir) : getDefaultSessionDir(cwd);
    const filterCwd = sessionDir !== void 0 && dir !== getDefaultSessionDirPath(cwd);
    const mostRecent = findMostRecentSession(dir, filterCwd ? cwd : void 0);
    if (mostRecent) {
      return new _SessionManager(cwd, dir, mostRecent, true);
    }
    return new _SessionManager(cwd, dir, void 0, true);
  }
  /** Create an in-memory session (no file persistence), optionally from entries held outside the filesystem. */
  static inMemory(cwd = process.cwd(), options, entries) {
    return new _SessionManager(cwd, "", void 0, false, options, entries);
  }
  /**
   * Fork a session from another project directory into the current project.
   * Creates a new session in the target cwd with the full history from the source session.
   * @param sourcePath Path to the source session file
   * @param targetCwd Target working directory (where the new session will be stored)
   * @param sessionDir Optional session directory. If omitted, uses default for targetCwd.
   */
  static forkFrom(sourcePath, targetCwd, sessionDir, options) {
    const resolvedSourcePath = resolvePath(sourcePath);
    const resolvedTargetCwd = resolvePath(targetCwd);
    const sourceEntries = loadEntriesFromFile(resolvedSourcePath);
    if (sourceEntries.length === 0) {
      throw new Error(`Cannot fork: source session file is empty or invalid: ${resolvedSourcePath}`);
    }
    const sourceHeader = sourceEntries.find((e) => e.type === "session");
    if (!sourceHeader) {
      throw new Error(`Cannot fork: source session has no header: ${resolvedSourcePath}`);
    }
    const dir = sessionDir ? normalizePath(sessionDir) : getDefaultSessionDir(resolvedTargetCwd);
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }
    if (options?.id !== void 0) {
      assertValidSessionId(options.id);
    }
    const newSessionId = options?.id ?? createSessionId();
    const timestamp = (/* @__PURE__ */ new Date()).toISOString();
    const fileTimestamp = timestamp.replace(/[:.]/g, "-");
    const newSessionFile = join2(dir, `${fileTimestamp}_${newSessionId}.jsonl`);
    const newHeader = {
      type: "session",
      version: CURRENT_SESSION_VERSION,
      id: newSessionId,
      timestamp,
      cwd: resolvedTargetCwd,
      parentSession: resolvedSourcePath
    };
    writeFileSync(newSessionFile, `${JSON.stringify(newHeader)}
`, { flag: "wx" });
    for (const entry of sourceEntries) {
      if (entry.type !== "session") {
        appendFileSync(newSessionFile, `${JSON.stringify(entry)}
`);
      }
    }
    return new _SessionManager(resolvedTargetCwd, dir, newSessionFile, true);
  }
  /**
   * Find an exact session ID without loading transcript bodies.
   * @param cwd Working directory (used to compute default session directory)
   * @param id Exact session ID
   * @param sessionDir Optional session directory. If omitted, uses default (~/.pi/agent/sessions/<encoded-cwd>/).
   */
  static findById(cwd, id, sessionDir) {
    const dir = sessionDir ? normalizePath(sessionDir) : getDefaultSessionDir(cwd);
    const filterCwd = sessionDir !== void 0 && dir !== getDefaultSessionDirPath(cwd);
    const resolvedCwd = resolvePath(cwd);
    try {
      for (const file of readdirSync(dir)) {
        if (!file.endsWith(".jsonl"))
          continue;
        const path4 = join2(dir, file);
        const header = readSessionHeaderForDiscovery(path4);
        if (header?.id !== id)
          continue;
        if (filterCwd && !sessionCwdMatches(getSessionHeaderCwd(header), resolvedCwd))
          continue;
        return path4;
      }
    } catch {
    }
    return void 0;
  }
  /**
   * List all sessions for a directory.
   * @param cwd Working directory (used to compute default session directory)
   * @param sessionDir Optional session directory. If omitted, uses default (~/.pi/agent/sessions/<encoded-cwd>/).
   * @param onProgress Optional callback for progress updates (loaded, total)
   */
  static async list(cwd, sessionDir, onProgress, signal) {
    const dir = sessionDir ? normalizePath(sessionDir) : getDefaultSessionDir(cwd);
    const filterCwd = sessionDir !== void 0 && dir !== getDefaultSessionDirPath(cwd);
    const resolvedCwd = resolvePath(cwd);
    const includeSession = (session) => !filterCwd || sessionCwdMatches(session.cwd, resolvedCwd);
    const progress = onProgress ? (loaded, total, partialSessions) => onProgress(loaded, total, partialSessions?.filter(includeSession)) : void 0;
    const sessions = (await listSessionsFromDir(dir, progress, signal)).filter(includeSession);
    return sortSessionInfos(sessions);
  }
  static async listAll(sessionDirOrOnProgress, onProgressOrSignal, signal) {
    const customSessionDir = typeof sessionDirOrOnProgress === "string" ? normalizePath(sessionDirOrOnProgress) : void 0;
    const progress = typeof sessionDirOrOnProgress === "function" ? sessionDirOrOnProgress : typeof onProgressOrSignal === "function" ? onProgressOrSignal : void 0;
    const abortSignal = typeof sessionDirOrOnProgress === "string" || typeof onProgressOrSignal === "function" ? signal : onProgressOrSignal ?? signal;
    abortSignal?.throwIfAborted();
    if (customSessionDir) {
      return sortSessionInfos(await listSessionsFromDir(customSessionDir, progress, abortSignal));
    }
    const sessionsDir = (0, config_exports.getSessionsDir)();
    try {
      if (!existsSync(sessionsDir))
        return [];
      const entries = await readdir(sessionsDir, { withFileTypes: true });
      const dirs = entries.filter((entry) => entry.isDirectory() || entry.isSymbolicLink()).map((entry) => join2(sessionsDir, entry.name));
      const dirFiles = await mapWithConcurrency(dirs, MAX_CONCURRENT_SESSION_DISCOVERY_LOADS, async (dir) => {
        try {
          return (await readdir(dir)).filter((file) => file.endsWith(".jsonl")).map((file) => join2(dir, file));
        } catch {
          return [];
        }
      }, abortSignal);
      const allFiles = dirFiles.flat();
      const candidates = await mapWithConcurrency(allFiles, MAX_CONCURRENT_SESSION_DISCOVERY_LOADS, async (path4) => {
        try {
          return { path: path4, stats: await stat(path4) };
        } catch {
          return { path: path4 };
        }
      }, abortSignal);
      candidates.sort((a, b) => (b.stats?.mtimeMs ?? Number.NEGATIVE_INFINITY) - (a.stats?.mtimeMs ?? Number.NEGATIVE_INFINITY) || basename(b.path).localeCompare(basename(a.path)));
      const totalFiles = candidates.length;
      let loaded = 0;
      let firstCandidateLoaded = false;
      const partialSessions = [];
      const results = await buildSessionInfosWithConcurrency(candidates, (info, index) => {
        loaded++;
        if (index === 0)
          firstCandidateLoaded = true;
        if (info)
          partialSessions.push(info);
        const publishPartial = firstCandidateLoaded && (index === 0 || loaded % ALL_SESSION_LIST_PUBLISH_INTERVAL === 0 || loaded === totalFiles);
        progress?.(loaded, totalFiles, publishPartial ? sortSessionInfos([...partialSessions]) : void 0);
      }, abortSignal);
      return sortSessionInfos(results.filter((info) => info !== null));
    } catch {
      abortSignal?.throwIfAborted();
      return [];
    }
  }
};

// target/vendor-pi/dist/core/compaction/branch-summarization.js
import { contentText as contentText3, normalizeContext as normalizeContext2 } from "@earendil-works/pi-ai";

// target/vendor-pi/dist/core/compaction/compaction.js
import { contentText as contentText2, getCurrentSystemMessage as getCurrentSystemMessage2, normalizeContext, retryAssistantCall, uuidv7 as uuidv72 } from "@earendil-works/pi-ai";
import { completeSimple } from "@earendil-works/pi-ai/compat";

// target/vendor-pi/dist/core/usage-totals.js
function createUsageTotals() {
  return {
    input: 0,
    output: 0,
    cacheRead: 0,
    cacheWrite: 0,
    cost: 0
  };
}
function addUsageToTotals(totals, usage) {
  totals.input += usage.input;
  totals.output += usage.output;
  totals.cacheRead += usage.cacheRead;
  totals.cacheWrite += usage.cacheWrite;
  totals.cost += usage.cost.total;
}
function combineUsage(first, second) {
  return {
    input: first.input + second.input,
    output: first.output + second.output,
    cacheRead: first.cacheRead + second.cacheRead,
    cacheWrite: first.cacheWrite + second.cacheWrite,
    ...first.cacheWrite1h !== void 0 || second.cacheWrite1h !== void 0 ? { cacheWrite1h: (first.cacheWrite1h ?? 0) + (second.cacheWrite1h ?? 0) } : {},
    ...first.reasoning !== void 0 || second.reasoning !== void 0 ? { reasoning: (first.reasoning ?? 0) + (second.reasoning ?? 0) } : {},
    totalTokens: first.totalTokens + second.totalTokens,
    cost: {
      input: first.cost.input + second.cost.input,
      output: first.cost.output + second.cost.output,
      cacheRead: first.cost.cacheRead + second.cost.cacheRead,
      cacheWrite: first.cost.cacheWrite + second.cost.cacheWrite,
      total: first.cost.total + second.cost.total
    }
  };
}

// target/vendor-pi/dist/core/compaction/utils.js
import { contentText } from "@earendil-works/pi-ai";
function createFileOps() {
  return {
    read: /* @__PURE__ */ new Set(),
    written: /* @__PURE__ */ new Set(),
    edited: /* @__PURE__ */ new Set()
  };
}
function extractFileOpsFromMessage(message, fileOps) {
  if (message.role === "toolResult") {
    for (const call of message.nestedCalls?.calls ?? [])
      addFileOp(call.name, call.arguments, fileOps);
    return;
  }
  if (message.role !== "assistant")
    return;
  if (!("content" in message) || !Array.isArray(message.content))
    return;
  for (const block of message.content) {
    if (typeof block !== "object" || block === null)
      continue;
    if (!("type" in block) || block.type !== "toolCall")
      continue;
    if (!("arguments" in block) || !("name" in block))
      continue;
    addFileOp(block.name, block.arguments, fileOps);
  }
}
function addFileOp(toolName, args, fileOps) {
  const path4 = typeof args?.path === "string" ? args.path : void 0;
  if (!path4)
    return;
  switch (toolName) {
    case "read":
      fileOps.read.add(path4);
      break;
    case "write":
      fileOps.written.add(path4);
      break;
    case "edit":
      fileOps.edited.add(path4);
      break;
  }
}
function computeFileLists(fileOps) {
  const modified = /* @__PURE__ */ new Set([...fileOps.edited, ...fileOps.written]);
  const readOnly = [...fileOps.read].filter((f) => !modified.has(f)).sort();
  const modifiedFiles = [...modified].sort();
  return { readFiles: readOnly, modifiedFiles };
}
function formatFileOperations(readFiles, modifiedFiles) {
  const sections = [];
  if (readFiles.length > 0) {
    sections.push(`<read-files>
${readFiles.join("\n")}
</read-files>`);
  }
  if (modifiedFiles.length > 0) {
    sections.push(`<modified-files>
${modifiedFiles.join("\n")}
</modified-files>`);
  }
  if (sections.length === 0)
    return "";
  return `

${sections.join("\n\n")}`;
}
var TOOL_RESULT_MAX_CHARS = 2e3;
function truncateForSummary(text, maxChars) {
  if (text.length <= maxChars)
    return text;
  const truncatedChars = text.length - maxChars;
  return `${text.slice(0, maxChars)}

[... ${truncatedChars} more characters truncated]`;
}
function serializeConversation(messages) {
  const parts = [];
  for (const msg of messages) {
    if (msg.role === "user") {
      const content = contentText(msg.content, "");
      if (content)
        parts.push(`[User]: ${content}`);
    } else if (msg.role === "assistant") {
      const thinkingParts = [];
      const toolCalls = [];
      for (const block of msg.content) {
        if (block.type === "thinking") {
          thinkingParts.push(block.thinking);
        } else if (block.type === "toolCall") {
          const args = block.arguments;
          const argsStr = Object.entries(args).map(([k, v]) => `${k}=${JSON.stringify(v)}`).join(", ");
          toolCalls.push(`${block.name}(${argsStr})`);
        }
      }
      if (thinkingParts.length > 0) {
        parts.push(`[Assistant thinking]: ${thinkingParts.join("\n")}`);
      }
      if (msg.content.some((block) => block.type === "text")) {
        parts.push(`[Assistant]: ${contentText(msg.content)}`);
      }
      if (toolCalls.length > 0) {
        parts.push(`[Assistant tool calls]: ${toolCalls.join("; ")}`);
      }
    } else if (msg.role === "toolResult") {
      const content = contentText(msg.content, "");
      if (content) {
        parts.push(`[Tool result]: ${truncateForSummary(content, TOOL_RESULT_MAX_CHARS)}`);
      }
    }
  }
  return parts.join("\n\n");
}
var SUMMARIZATION_SYSTEM_PROMPT = `You are a context summarization assistant. Your task is to read a conversation between a user and an AI assistant, then produce a structured summary following the exact format specified.

Do NOT continue the conversation. Do NOT respond to any questions in the conversation. ONLY output the structured summary.`;

// target/vendor-pi/dist/core/compaction/compaction.js
function calculateContextTokens(usage) {
  return usage.totalTokens || usage.input + usage.output + usage.cacheRead + usage.cacheWrite;
}
function getAssistantUsage(msg) {
  if (msg.role === "assistant" && "usage" in msg) {
    const assistantMsg = msg;
    if (assistantMsg.stopReason !== "aborted" && assistantMsg.stopReason !== "error" && assistantMsg.usage && calculateContextTokens(assistantMsg.usage) > 0) {
      return assistantMsg.usage;
    }
  }
  return void 0;
}
function getLastAssistantUsage(entries) {
  for (let i = entries.length - 1; i >= 0; i--) {
    const entry = entries[i];
    if (entry.type === "message") {
      const usage = getAssistantUsage(entry.message);
      if (usage)
        return usage;
    }
  }
  return void 0;
}
function shouldCompact(contextTokens, contextWindow, settings) {
  if (!settings.enabled)
    return false;
  return contextTokens > contextWindow - settings.reserveTokens;
}
var ESTIMATED_IMAGE_CHARS = 4800;
function estimateTextAndImageContentChars(content) {
  if (typeof content === "string") {
    return content.length;
  }
  let chars = 0;
  for (const block of content) {
    if (block.type === "text" && block.text) {
      chars += block.text.length;
    } else if (block.type === "image") {
      chars += ESTIMATED_IMAGE_CHARS;
    }
  }
  return chars;
}
function estimateTokens(message) {
  let chars = 0;
  switch (message.role) {
    case "system": {
      const system = message;
      chars = estimateTextAndImageContentChars(system.content);
      if (system.sections) {
        for (const section of Object.values(system.sections)) {
          if (section)
            chars += section.length;
        }
      }
      if (system.toolsAdded)
        chars += JSON.stringify(system.toolsAdded).length;
      return Math.ceil(chars / 4);
    }
    case "user": {
      chars = estimateTextAndImageContentChars(message.content);
      return Math.ceil(chars / 4);
    }
    case "assistant": {
      const assistant = message;
      for (const block of assistant.content) {
        if (block.type === "text") {
          chars += block.text.length;
        } else if (block.type === "thinking") {
          chars += block.thinking.length;
        } else if (block.type === "toolCall") {
          chars += block.name.length + JSON.stringify(block.arguments).length;
        }
      }
      return Math.ceil(chars / 4);
    }
    case "custom":
    case "toolResult": {
      chars = estimateTextAndImageContentChars(message.content);
      return Math.ceil(chars / 4);
    }
    case "bashExecution": {
      chars = message.command.length + message.output.length;
      return Math.ceil(chars / 4);
    }
    case "branchSummary":
    case "compactionSummary": {
      chars = message.summary.length;
      return Math.ceil(chars / 4);
    }
  }
  return 0;
}
function isCutPointMessage(message) {
  switch (message.role) {
    case "user":
    case "assistant":
    case "bashExecution":
    case "custom":
    case "branchSummary":
    case "compactionSummary":
      return true;
    case "toolResult":
      return false;
  }
  return false;
}
function isTurnStartMessage(message) {
  switch (message.role) {
    case "user":
    case "bashExecution":
    case "custom":
    case "branchSummary":
    case "compactionSummary":
      return true;
    case "assistant":
    case "toolResult":
      return false;
  }
  return false;
}
function isTurnStartEntry(entry) {
  if (entry.type === "compaction") {
    return false;
  }
  return sessionEntryToContextMessages(entry).some(isTurnStartMessage);
}
function findValidCutPoints(entries, startIndex, endIndex) {
  const cutPoints = [];
  for (let i = startIndex; i < endIndex; i++) {
    const entry = entries[i];
    if (entry.type === "compaction") {
      continue;
    }
    if (sessionEntryToContextMessages(entry).some(isCutPointMessage)) {
      cutPoints.push(i);
    }
  }
  return cutPoints;
}
function findTurnStartIndex(entries, entryIndex, startIndex) {
  for (let i = entryIndex; i >= startIndex; i--) {
    if (isTurnStartEntry(entries[i])) {
      return i;
    }
  }
  return -1;
}
function findCutPoint(entries, startIndex, endIndex, keepRecentTokens) {
  const cutPoints = findValidCutPoints(entries, startIndex, endIndex);
  if (cutPoints.length === 0) {
    return { firstKeptEntryIndex: startIndex, turnStartIndex: -1, isSplitTurn: false };
  }
  let accumulatedTokens = 0;
  let cutIndex = cutPoints[0];
  for (let i = endIndex - 1; i >= startIndex; i--) {
    const entry = entries[i];
    const messageTokens = sessionEntryToContextMessages(entry).reduce((sum, message) => sum + estimateTokens(message), 0);
    if (messageTokens === 0)
      continue;
    accumulatedTokens += messageTokens;
    if (accumulatedTokens >= keepRecentTokens) {
      cutIndex = cutPoints.find((candidate) => candidate >= i) ?? cutPoints[cutPoints.length - 1];
      break;
    }
  }
  while (cutIndex > startIndex) {
    const prevEntry = entries[cutIndex - 1];
    if (prevEntry.type === "compaction" || sessionEntryToContextMessages(prevEntry).length > 0) {
      break;
    }
    cutIndex--;
  }
  const cutEntry = entries[cutIndex];
  const startsTurn = isTurnStartEntry(cutEntry);
  const turnStartIndex = startsTurn ? -1 : findTurnStartIndex(entries, cutIndex, startIndex);
  return {
    firstKeptEntryIndex: cutIndex,
    turnStartIndex,
    isSplitTurn: !startsTurn && turnStartIndex !== -1
  };
}
var SUMMARIZATION_PROMPT = `The messages above are a conversation to summarize. Create a structured context checkpoint summary that another LLM will use to continue the work.

Use this EXACT format:

## Goal
[What is the user trying to accomplish? Can be multiple items if the session covers different tasks.]

## Constraints & Preferences
- [Any constraints, preferences, or requirements mentioned by user]
- [Or "(none)" if none were mentioned]

## Progress
### Done
- [x] [Completed tasks/changes]

### In Progress
- [ ] [Current work]

### Blocked
- [Issues preventing progress, if any]

## Key Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [Ordered list of what should happen next]

## Critical Context
- [Any data, examples, or references needed to continue]
- [Or "(none)" if not applicable]

Keep each section concise. Preserve exact file paths, function names, and error messages.`;
var UPDATE_SUMMARIZATION_INSTRUCTIONS = `Update the existing structured summary with new information. RULES:
- PRESERVE all existing information from the previous summary
- ADD new progress, decisions, and context from the new messages
- UPDATE the Progress section: move items from "In Progress" to "Done" when completed
- UPDATE "Next Steps" based on what was accomplished
- PRESERVE exact file paths, function names, and error messages
- If something is no longer relevant, you may remove it

Use this EXACT format:

## Goal
[Preserve existing goals, add new ones if the task expanded]

## Constraints & Preferences
- [Preserve existing, add new ones discovered]

## Progress
### Done
- [x] [Include previously done items AND newly completed items]

### In Progress
- [ ] [Current work - update based on progress]

### Blocked
- [Current blockers - remove if resolved]

## Key Decisions
- **[Decision]**: [Brief rationale] (preserve all previous, add new)

## Next Steps
1. [Update based on current state]

## Critical Context
- [Preserve important context, add new if needed]

Keep each section concise. Preserve exact file paths, function names, and error messages.`;
var UPDATE_SUMMARIZATION_PROMPT = `The messages above are NEW conversation messages to incorporate into the existing summary provided in <previous-summary> tags.

${UPDATE_SUMMARIZATION_INSTRUCTIONS}`;
function getSummarizationFailure(response, label) {
  if (response.stopReason === "error") {
    return `${label} failed: ${response.errorMessage || "Unknown error"}`;
  }
  if (response.stopReason === "length") {
    return `${label} failed: generation hit the token cap and the summary is incomplete`;
  }
  return void 0;
}
function createSummarizationOptions(model, maxTokens, apiKey, headers, env, signal, thinkingLevel, sessionId) {
  const options = { maxTokens, signal, apiKey, headers, env, sessionId };
  if (model.reasoning && thinkingLevel && thinkingLevel !== "off") {
    options.reasoning = thinkingLevel;
  }
  return options;
}
async function completeSummarization(model, context, options, streamFn, retry, callbacks) {
  const requestOptions = {
    ...options,
    cacheRetention: "none",
    sessionId: options.sessionId ?? uuidv72()
  };
  const produce = async () => streamFn ? (await streamFn(model, context, requestOptions)).result() : completeSimple(model, context, requestOptions);
  return retryAssistantCall(produce, retry, requestOptions.signal, callbacks);
}
async function generateSummary(currentMessages, model, reserveTokens, apiKey, headers, signal, customInstructions, previousSummary, thinkingLevel, streamFn, env, retry, callbacks, sessionId) {
  return (await generateSummaryWithUsage(currentMessages, model, reserveTokens, apiKey, headers, signal, customInstructions, previousSummary, thinkingLevel, streamFn, env, retry, callbacks, sessionId)).text;
}
function buildSummarizationContext(promptText) {
  return normalizeContext({
    systemPrompt: SUMMARIZATION_SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: [{ type: "text", text: promptText }],
        timestamp: Date.now()
      }
    ]
  });
}
async function generateSummaryWithUsage(currentMessages, model, reserveTokens, apiKey, headers, signal, customInstructions, previousSummary, thinkingLevel, streamFn, env, retry, callbacks, sessionId) {
  const maxTokens = Math.min(Math.floor(0.8 * reserveTokens), model.maxTokens > 0 ? model.maxTokens : Number.POSITIVE_INFINITY);
  let basePrompt = previousSummary ? UPDATE_SUMMARIZATION_PROMPT : SUMMARIZATION_PROMPT;
  if (customInstructions) {
    basePrompt = `${basePrompt}

Additional focus: ${customInstructions}`;
  }
  const llmMessages = convertToLlm(currentMessages);
  const conversationText = serializeConversation(llmMessages);
  let promptText = `<conversation>
${conversationText}
</conversation>

`;
  if (previousSummary) {
    promptText += `<previous-summary>
${previousSummary}
</previous-summary>

`;
  }
  promptText += basePrompt;
  const completionOptions = createSummarizationOptions(model, maxTokens, apiKey, headers, env, signal, thinkingLevel, sessionId);
  const response = await completeSummarization(model, buildSummarizationContext(promptText), completionOptions, streamFn, retry, callbacks);
  const failure = getSummarizationFailure(response, "Summarization");
  if (failure) {
    throw new Error(failure);
  }
  if (response.content.some((block) => block.type === "toolCall")) {
    throw new Error("Summarization attempted to call a tool");
  }
  const textContent = contentText2(response.content);
  return { text: textContent, usage: response.usage };
}
var TURN_PREFIX_SUMMARIZATION_PROMPT = `The messages above are earlier context from an ongoing conversation. Later messages are stored separately and do not need to be reconstructed.

Create a concise checkpoint of the user's request and the progress shown above. This checkpoint will be placed before the later messages so the conversation can continue with the necessary context.

## Original Request
[What did the user ask for?]

## Progress So Far
- [Key decisions and work completed in these messages]

## Context Needed to Continue
- [Information from these messages needed to understand the later work]

Only summarize information explicitly present above. Do not infer or recreate later messages.`;
async function compact(preparation, model, apiKey, headers, customInstructions, signal, thinkingLevel, streamFn, env, retry, callbacks, sessionId) {
  const { firstKeptEntryId, messagesToSummarize, turnPrefixMessages, isSplitTurn, tokensBefore, previousSummary, fileOps, settings } = preparation;
  let summary;
  let summaryUsage;
  if (isSplitTurn && turnPrefixMessages.length > 0) {
    let historyText = previousSummary ?? "No prior history.";
    let historyUsage;
    if (messagesToSummarize.length > 0) {
      const historyResult = await generateSummaryWithUsage(messagesToSummarize, model, settings.reserveTokens, apiKey, headers, signal, customInstructions, previousSummary, thinkingLevel, streamFn, env, retry, callbacks, sessionId);
      historyText = historyResult.text;
      historyUsage = historyResult.usage;
    }
    const turnPrefixResult = await generateTurnPrefixSummary(turnPrefixMessages, model, settings.reserveTokens, apiKey, headers, env, signal, thinkingLevel, streamFn, retry, callbacks, sessionId);
    summary = `${historyText}

---

**Turn Context (split turn):**

${turnPrefixResult.text}`;
    summaryUsage = historyUsage ? combineUsage(historyUsage, turnPrefixResult.usage) : turnPrefixResult.usage;
  } else {
    const result = await generateSummaryWithUsage(messagesToSummarize, model, settings.reserveTokens, apiKey, headers, signal, customInstructions, previousSummary, thinkingLevel, streamFn, env, retry, callbacks, sessionId);
    summary = result.text;
    summaryUsage = result.usage;
  }
  const { readFiles, modifiedFiles } = computeFileLists(fileOps);
  summary += formatFileOperations(readFiles, modifiedFiles);
  if (!firstKeptEntryId) {
    throw new Error("First kept entry has no UUID - session may need migration");
  }
  return {
    summary,
    firstKeptEntryId,
    tokensBefore,
    usage: summaryUsage,
    details: { readFiles, modifiedFiles }
  };
}
async function generateTurnPrefixSummary(messages, model, reserveTokens, apiKey, headers, env, signal, thinkingLevel, streamFn, retry, callbacks, sessionId) {
  const maxTokens = Math.min(Math.floor(0.5 * reserveTokens), model.maxTokens > 0 ? model.maxTokens : Number.POSITIVE_INFINITY);
  const llmMessages = convertToLlm(messages);
  const conversationText = serializeConversation(llmMessages);
  const promptText = `# Conversation
${conversationText}

# Instructions
${TURN_PREFIX_SUMMARIZATION_PROMPT}`;
  const response = await completeSummarization(model, buildSummarizationContext(promptText), createSummarizationOptions(model, maxTokens, apiKey, headers, env, signal, thinkingLevel, sessionId), streamFn, retry, callbacks);
  const failure = getSummarizationFailure(response, "Turn prefix summarization");
  if (failure) {
    throw new Error(failure);
  }
  if (response.content.some((block) => block.type === "toolCall")) {
    throw new Error("Turn prefix summarization attempted to call a tool");
  }
  return {
    text: contentText2(response.content),
    usage: response.usage
  };
}

// target/vendor-pi/dist/core/compaction/branch-summarization.js
function collectEntriesForBranchSummary(session, oldLeafId, targetId) {
  if (!oldLeafId) {
    return { entries: [], commonAncestorId: null };
  }
  const oldPath = new Set(session.getBranch(oldLeafId).map((e) => e.id));
  const targetPath = session.getBranch(targetId);
  let commonAncestorId = null;
  for (let i = targetPath.length - 1; i >= 0; i--) {
    if (oldPath.has(targetPath[i].id)) {
      commonAncestorId = targetPath[i].id;
      break;
    }
  }
  const entries = [];
  let current = oldLeafId;
  while (current && current !== commonAncestorId) {
    const entry = session.getEntry(current);
    if (!entry)
      break;
    entries.push(entry);
    current = entry.parentId;
  }
  entries.reverse();
  return { entries, commonAncestorId };
}
function getMessageFromEntry(entry) {
  switch (entry.type) {
    case "message":
      if (entry.message.role === "toolResult")
        return void 0;
      return entry.message;
    case "custom_message":
      return createCustomMessage(entry.customType, entry.content, entry.display, entry.details, entry.timestamp);
    case "branch_summary":
      return createBranchSummaryMessage(entry.summary, entry.fromId, entry.timestamp);
    case "compaction":
      return createCompactionSummaryMessage(entry.summary, entry.tokensBefore, entry.timestamp);
    // These don't contribute to conversation content
    case "thinking_level_change":
    case "model_change":
    case "custom":
    case "label":
    case "session_info":
      return void 0;
  }
}
function prepareBranchEntries(entries, tokenBudget = 0) {
  const messages = [];
  const fileOps = createFileOps();
  let totalTokens = 0;
  for (const entry of entries) {
    if (entry.type === "branch_summary" && !entry.fromHook && entry.details) {
      const details = entry.details;
      if (Array.isArray(details.readFiles)) {
        for (const f of details.readFiles)
          fileOps.read.add(f);
      }
      if (Array.isArray(details.modifiedFiles)) {
        for (const f of details.modifiedFiles) {
          fileOps.edited.add(f);
        }
      }
    }
  }
  for (let i = entries.length - 1; i >= 0; i--) {
    const entry = entries[i];
    const message = getMessageFromEntry(entry);
    if (!message)
      continue;
    extractFileOpsFromMessage(message, fileOps);
    const tokens = estimateTokens(message);
    if (tokenBudget > 0 && totalTokens + tokens > tokenBudget) {
      if (entry.type === "compaction" || entry.type === "branch_summary") {
        if (totalTokens < tokenBudget * 0.9) {
          messages.unshift(message);
          totalTokens += tokens;
        }
      }
      break;
    }
    messages.unshift(message);
    totalTokens += tokens;
  }
  return { messages, fileOps, totalTokens };
}
var BRANCH_SUMMARY_PREAMBLE = `The user explored a different conversation branch before returning here.
Summary of that exploration:

`;
var BRANCH_SUMMARY_PROMPT = `Create a structured summary of this conversation branch for context when returning later.

Use this EXACT format:

## Goal
[What was the user trying to accomplish in this branch?]

## Constraints & Preferences
- [Any constraints, preferences, or requirements mentioned]
- [Or "(none)" if none were mentioned]

## Progress
### Done
- [x] [Completed tasks/changes]

### In Progress
- [ ] [Work that was started but not finished]

### Blocked
- [Issues preventing progress, if any]

## Key Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [What should happen next to continue this work]

Keep each section concise. Preserve exact file paths, function names, and error messages.`;
async function generateBranchSummary(entries, options) {
  const { model, apiKey, headers, env, signal, customInstructions, replaceInstructions, reserveTokens = 16384, streamFn, retry, callbacks } = options;
  const contextWindow = model.contextWindow || 128e3;
  const tokenBudget = contextWindow - reserveTokens;
  const { messages, fileOps } = prepareBranchEntries(entries, tokenBudget);
  if (messages.length === 0) {
    return { summary: "No content to summarize" };
  }
  const llmMessages = convertToLlm(messages);
  const conversationText = serializeConversation(llmMessages);
  let instructions;
  if (replaceInstructions && customInstructions) {
    instructions = customInstructions;
  } else if (customInstructions) {
    instructions = `${BRANCH_SUMMARY_PROMPT}

Additional focus: ${customInstructions}`;
  } else {
    instructions = BRANCH_SUMMARY_PROMPT;
  }
  const promptText = `<conversation>
${conversationText}
</conversation>

${instructions}`;
  const summarizationMessages = [
    {
      role: "user",
      content: [{ type: "text", text: promptText }],
      timestamp: Date.now()
    }
  ];
  const maxTokens = Math.min(4096, model.maxTokens > 0 ? model.maxTokens : Number.POSITIVE_INFINITY);
  const context = normalizeContext2({ systemPrompt: SUMMARIZATION_SYSTEM_PROMPT, messages: summarizationMessages });
  const requestOptions = { apiKey, headers, env, signal, maxTokens };
  const response = await completeSummarization(model, context, requestOptions, streamFn, retry, callbacks);
  if (response.stopReason === "aborted") {
    return { aborted: true };
  }
  const failure = getSummarizationFailure(response, "Branch summarization");
  if (failure) {
    return { error: failure };
  }
  if (response.content.some((block) => block.type === "toolCall")) {
    return { error: "Branch summarization attempted to call a tool" };
  }
  let summary = contentText3(response.content);
  summary = BRANCH_SUMMARY_PREAMBLE + summary;
  const { readFiles, modifiedFiles } = computeFileLists(fileOps);
  summary += formatFileOperations(readFiles, modifiedFiles);
  return {
    summary: summary || "No summary generated",
    usage: response.usage,
    readFiles,
    modifiedFiles
  };
}

// target/vendor-pi/dist/core/skills.js
var import_ignore = __toESM(require_ignore());
import { existsSync as existsSync2, readdirSync as readdirSync2, readFileSync, statSync as statSync3 } from "fs";
import { basename as basename2, dirname, join as join3, relative as relative2, resolve as resolve2, sep as sep2 } from "path";

// target/vendor-pi/dist/utils/frontmatter.js
var frontmatter_exports = {};
__reExport(frontmatter_exports, host_star2);
import * as host_star2 from "yapi:pi/host";

// target/vendor-pi/dist/core/source-info.js
function createSyntheticSourceInfo(path4, options) {
  return {
    path: path4,
    source: options.source,
    scope: options.scope ?? "temporary",
    origin: options.origin ?? "top-level",
    baseDir: options.baseDir
  };
}

// target/vendor-pi/dist/core/skills.js
var MAX_NAME_LENGTH = 64;
var MAX_DESCRIPTION_LENGTH = 1024;
var IGNORE_FILE_NAMES = [".gitignore", ".ignore", ".fdignore"];
function toPosixPath(p) {
  return p.split(sep2).join("/");
}
function prefixIgnorePattern(line, prefix) {
  const trimmed = line.trim();
  if (!trimmed)
    return null;
  if (trimmed.startsWith("#") && !trimmed.startsWith("\\#"))
    return null;
  let pattern = line;
  let negated = false;
  if (pattern.startsWith("!")) {
    negated = true;
    pattern = pattern.slice(1);
  } else if (pattern.startsWith("\\!")) {
    pattern = pattern.slice(1);
  }
  if (pattern.startsWith("/")) {
    pattern = pattern.slice(1);
  }
  const prefixed = prefix ? `${prefix}${pattern}` : pattern;
  return negated ? `!${prefixed}` : prefixed;
}
function addIgnoreRules(ig, dir, rootDir) {
  const relativeDir = relative2(rootDir, dir);
  const prefix = relativeDir ? `${toPosixPath(relativeDir)}/` : "";
  for (const filename of IGNORE_FILE_NAMES) {
    const ignorePath = join3(dir, filename);
    if (!existsSync2(ignorePath))
      continue;
    try {
      const content = readFileSync(ignorePath, "utf-8");
      const patterns = content.split(/\r?\n/).map((line) => prefixIgnorePattern(line, prefix)).filter((line) => Boolean(line));
      if (patterns.length > 0) {
        ig.add(patterns);
      }
    } catch {
    }
  }
}
function validateName(name) {
  const errors = [];
  if (name.length > MAX_NAME_LENGTH) {
    errors.push(`name exceeds ${MAX_NAME_LENGTH} characters (${name.length})`);
  }
  if (!/^[a-z0-9-]+$/.test(name)) {
    errors.push(`name contains invalid characters (must be lowercase a-z, 0-9, hyphens only)`);
  }
  if (name.startsWith("-") || name.endsWith("-")) {
    errors.push(`name must not start or end with a hyphen`);
  }
  if (name.includes("--")) {
    errors.push(`name must not contain consecutive hyphens`);
  }
  return errors;
}
function validateDescription(description) {
  const errors = [];
  if (typeof description !== "string" || description.trim() === "") {
    errors.push("description is required");
  } else if (description.length > MAX_DESCRIPTION_LENGTH) {
    errors.push(`description exceeds ${MAX_DESCRIPTION_LENGTH} characters (${description.length})`);
  }
  return errors;
}
function createSkillSourceInfo(filePath, baseDir, source) {
  switch (source) {
    case "user":
      return createSyntheticSourceInfo(filePath, {
        source: "local",
        scope: "user",
        baseDir
      });
    case "project":
      return createSyntheticSourceInfo(filePath, {
        source: "local",
        scope: "project",
        baseDir
      });
    case "path":
      return createSyntheticSourceInfo(filePath, {
        source: "local",
        baseDir
      });
    default:
      return createSyntheticSourceInfo(filePath, { source, baseDir });
  }
}
function loadSkillsFromDir(options) {
  const { dir, source } = options;
  return loadSkillsFromDirInternal(dir, source, true);
}
function loadSkillsFromDirInternal(dir, source, includeRootFiles, ignoreMatcher, rootDir) {
  const skills = [];
  const diagnostics = [];
  if (!existsSync2(dir)) {
    return { skills, diagnostics };
  }
  const root = rootDir ?? dir;
  const ig = ignoreMatcher ?? (0, import_ignore.default)();
  addIgnoreRules(ig, dir, root);
  try {
    const entries = readdirSync2(dir, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.name !== "SKILL.md") {
        continue;
      }
      const fullPath = join3(dir, entry.name);
      let isFile = entry.isFile();
      if (entry.isSymbolicLink()) {
        try {
          isFile = statSync3(fullPath).isFile();
        } catch {
          continue;
        }
      }
      const relPath = toPosixPath(relative2(root, fullPath));
      if (!isFile || ig.ignores(relPath)) {
        continue;
      }
      const result = loadSkillFromFile(fullPath, source);
      if (result.skill) {
        skills.push(result.skill);
      }
      diagnostics.push(...result.diagnostics);
      return { skills, diagnostics };
    }
    for (const entry of entries) {
      if (entry.name.startsWith(".")) {
        continue;
      }
      if (entry.name === "node_modules") {
        continue;
      }
      const fullPath = join3(dir, entry.name);
      let isDirectory = entry.isDirectory();
      let isFile = entry.isFile();
      if (entry.isSymbolicLink()) {
        try {
          const stats = statSync3(fullPath);
          isDirectory = stats.isDirectory();
          isFile = stats.isFile();
        } catch {
          continue;
        }
      }
      const relPath = toPosixPath(relative2(root, fullPath));
      const ignorePath = isDirectory ? `${relPath}/` : relPath;
      if (ig.ignores(ignorePath)) {
        continue;
      }
      if (isDirectory) {
        const subResult = loadSkillsFromDirInternal(fullPath, source, false, ig, root);
        skills.push(...subResult.skills);
        diagnostics.push(...subResult.diagnostics);
        continue;
      }
      if (!isFile || !includeRootFiles || !entry.name.endsWith(".md")) {
        continue;
      }
      const result = loadSkillFromFile(fullPath, source);
      if (result.skill) {
        skills.push(result.skill);
      }
      diagnostics.push(...result.diagnostics);
    }
  } catch {
  }
  return { skills, diagnostics };
}
function loadSkillFromFile(filePath, source) {
  const diagnostics = [];
  const isDeclaredSkill = basename2(filePath) === "SKILL.md";
  let rawContent;
  try {
    rawContent = readFileSync(filePath, "utf-8");
  } catch (error) {
    const message = error instanceof Error ? error.message : "failed to read skill file";
    diagnostics.push({ type: "warning", message, path: filePath });
    return { skill: null, diagnostics };
  }
  let frontmatter;
  try {
    ({ frontmatter } = (0, frontmatter_exports.parseFrontmatter)(rawContent));
  } catch (error) {
    if (isDeclaredSkill) {
      const message = error instanceof Error ? error.message : "failed to parse skill file";
      diagnostics.push({ type: "warning", message, path: filePath });
    }
    return { skill: null, diagnostics };
  }
  const description = frontmatter.description;
  const hasDescription = typeof description === "string" && description.trim() !== "";
  if (!isDeclaredSkill && !hasDescription) {
    return { skill: null, diagnostics };
  }
  const skillDir = dirname(filePath);
  const parentDirName = basename2(skillDir);
  const descErrors = validateDescription(description);
  for (const error of descErrors) {
    diagnostics.push({ type: "warning", message: error, path: filePath });
  }
  const frontmatterName = typeof frontmatter.name === "string" ? frontmatter.name : void 0;
  const name = frontmatterName || parentDirName;
  const nameErrors = validateName(name);
  for (const error of nameErrors) {
    diagnostics.push({ type: "warning", message: error, path: filePath });
  }
  if (!hasDescription) {
    return { skill: null, diagnostics };
  }
  return {
    skill: {
      name,
      description,
      filePath,
      baseDir: skillDir,
      sourceInfo: createSkillSourceInfo(filePath, skillDir, source),
      disableModelInvocation: frontmatter["disable-model-invocation"] === true
    },
    diagnostics
  };
}
function formatSkillsForPrompt(skills, fileReadTool = "read") {
  const visibleSkills = skills.filter((s) => !s.disableModelInvocation);
  if (visibleSkills.length === 0) {
    return "";
  }
  const lines = [
    "\n\nThe following skills provide specialized instructions for specific tasks.",
    fileReadTool === "read" ? "Use the read tool to load a skill's file when the task matches its description." : "Use bash to load a skill's file when the task matches its description.",
    "When a skill file references a relative path, resolve it against the skill directory (parent of SKILL.md / dirname of the path) and use that absolute path in tool commands.",
    "",
    "<available_skills>"
  ];
  for (const skill of visibleSkills) {
    lines.push("  <skill>");
    lines.push(`    <name>${escapeXml(skill.name)}</name>`);
    lines.push(`    <description>${escapeXml(skill.description)}</description>`);
    lines.push(`    <location>${escapeXml(skill.filePath)}</location>`);
    lines.push("  </skill>");
  }
  lines.push("</available_skills>");
  return lines.join("\n");
}
function escapeXml(str2) {
  return str2.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&apos;");
}
function loadSkills(options) {
  const { agentDir, skillPaths, includeDefaults } = options;
  const resolvedCwd = resolvePath(options.cwd);
  const resolvedAgentDir = resolvePath(agentDir ?? (0, config_exports.getAgentDir)());
  const skillMap = /* @__PURE__ */ new Map();
  const realPathSet = /* @__PURE__ */ new Set();
  const allDiagnostics = [];
  const collisionDiagnostics = [];
  function addSkills(result) {
    allDiagnostics.push(...result.diagnostics);
    for (const skill of result.skills) {
      const realPath = canonicalizePath(skill.filePath);
      if (realPathSet.has(realPath)) {
        continue;
      }
      const existing = skillMap.get(skill.name);
      if (existing) {
        collisionDiagnostics.push({
          type: "collision",
          message: `name "${skill.name}" collision`,
          path: skill.filePath,
          collision: {
            resourceType: "skill",
            name: skill.name,
            winnerPath: existing.filePath,
            loserPath: skill.filePath
          }
        });
      } else {
        skillMap.set(skill.name, skill);
        realPathSet.add(realPath);
      }
    }
  }
  if (includeDefaults) {
    addSkills(loadSkillsFromDirInternal(join3(resolvedAgentDir, "skills"), "user", true));
    addSkills(loadSkillsFromDirInternal(resolve2(resolvedCwd, config_exports.CONFIG_DIR_NAME, "skills"), "project", true));
  }
  const userSkillsDir = join3(resolvedAgentDir, "skills");
  const projectSkillsDir = resolve2(resolvedCwd, config_exports.CONFIG_DIR_NAME, "skills");
  const isUnderPath = (target, root) => {
    const normalizedRoot = resolve2(root);
    if (target === normalizedRoot) {
      return true;
    }
    const prefix = normalizedRoot.endsWith(sep2) ? normalizedRoot : `${normalizedRoot}${sep2}`;
    return target.startsWith(prefix);
  };
  const getSource = (resolvedPath) => {
    if (!includeDefaults) {
      if (isUnderPath(resolvedPath, userSkillsDir))
        return "user";
      if (isUnderPath(resolvedPath, projectSkillsDir))
        return "project";
    }
    return "path";
  };
  for (const rawPath of skillPaths) {
    const resolvedPath = resolvePath(rawPath, resolvedCwd, { trim: true });
    if (!existsSync2(resolvedPath)) {
      allDiagnostics.push({ type: "warning", message: "skill path does not exist", path: resolvedPath });
      continue;
    }
    try {
      const stats = statSync3(resolvedPath);
      const source = getSource(resolvedPath);
      if (stats.isDirectory()) {
        addSkills(loadSkillsFromDirInternal(resolvedPath, source, true));
      } else if (stats.isFile() && resolvedPath.endsWith(".md")) {
        const result = loadSkillFromFile(resolvedPath, source);
        if (result.skill) {
          addSkills({ skills: [result.skill], diagnostics: result.diagnostics });
        } else {
          allDiagnostics.push(...result.diagnostics);
        }
      } else {
        allDiagnostics.push({ type: "warning", message: "skill path is not a markdown file", path: resolvedPath });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "failed to read skill path";
      allDiagnostics.push({ type: "warning", message, path: resolvedPath });
    }
  }
  return {
    skills: Array.from(skillMap.values()),
    diagnostics: [...allDiagnostics, ...collisionDiagnostics]
  };
}

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/diff/libesm/diff/base.js
var Diff = class {
  diff(oldStr, newStr, options = {}) {
    let callback;
    if (typeof options === "function") {
      callback = options;
      options = {};
    } else if ("callback" in options) {
      callback = options.callback;
    }
    const oldString = this.castInput(oldStr, options);
    const newString = this.castInput(newStr, options);
    const oldTokens = this.removeEmpty(this.tokenize(oldString, options));
    const newTokens = this.removeEmpty(this.tokenize(newString, options));
    return this.diffWithOptionsObj(oldTokens, newTokens, options, callback);
  }
  diffWithOptionsObj(oldTokens, newTokens, options, callback) {
    var _a2;
    const done = (value) => {
      value = this.postProcess(value, options);
      if (callback) {
        setTimeout(function() {
          callback(value);
        }, 0);
        return void 0;
      } else {
        return value;
      }
    };
    const newLen = newTokens.length, oldLen = oldTokens.length;
    let editLength = 1;
    let maxEditLength = newLen + oldLen;
    if (options.maxEditLength != null) {
      maxEditLength = Math.min(maxEditLength, options.maxEditLength);
    }
    const maxExecutionTime = (_a2 = options.timeout) !== null && _a2 !== void 0 ? _a2 : Infinity;
    const abortAfterTimestamp = Date.now() + maxExecutionTime;
    const bestPath = [{ oldPos: -1, lastComponent: void 0 }];
    let newPos = this.extractCommon(bestPath[0], newTokens, oldTokens, 0, options);
    if (bestPath[0].oldPos + 1 >= oldLen && newPos + 1 >= newLen) {
      return done(this.buildValues(bestPath[0].lastComponent, newTokens, oldTokens));
    }
    let minDiagonalToConsider = -Infinity, maxDiagonalToConsider = Infinity;
    const execEditLength = () => {
      for (let diagonalPath = Math.max(minDiagonalToConsider, -editLength); diagonalPath <= Math.min(maxDiagonalToConsider, editLength); diagonalPath += 2) {
        let basePath;
        const removePath = bestPath[diagonalPath - 1], addPath = bestPath[diagonalPath + 1];
        if (removePath) {
          bestPath[diagonalPath - 1] = void 0;
        }
        let canAdd = false;
        if (addPath) {
          const addPathNewPos = addPath.oldPos - diagonalPath;
          canAdd = addPath && 0 <= addPathNewPos && addPathNewPos < newLen;
        }
        const canRemove = removePath && removePath.oldPos + 1 < oldLen;
        if (!canAdd && !canRemove) {
          bestPath[diagonalPath] = void 0;
          continue;
        }
        if (!canRemove || canAdd && removePath.oldPos < addPath.oldPos) {
          basePath = this.addToPath(addPath, true, false, 0, options);
        } else {
          basePath = this.addToPath(removePath, false, true, 1, options);
        }
        newPos = this.extractCommon(basePath, newTokens, oldTokens, diagonalPath, options);
        if (basePath.oldPos + 1 >= oldLen && newPos + 1 >= newLen) {
          return done(this.buildValues(basePath.lastComponent, newTokens, oldTokens)) || true;
        } else {
          bestPath[diagonalPath] = basePath;
          if (basePath.oldPos + 1 >= oldLen) {
            maxDiagonalToConsider = Math.min(maxDiagonalToConsider, diagonalPath - 1);
          }
          if (newPos + 1 >= newLen) {
            minDiagonalToConsider = Math.max(minDiagonalToConsider, diagonalPath + 1);
          }
        }
      }
      editLength++;
    };
    if (callback) {
      (function exec() {
        setTimeout(function() {
          if (editLength > maxEditLength || Date.now() > abortAfterTimestamp) {
            return callback(void 0);
          }
          if (!execEditLength()) {
            exec();
          }
        }, 0);
      })();
    } else {
      while (editLength <= maxEditLength && Date.now() <= abortAfterTimestamp) {
        const ret = execEditLength();
        if (ret) {
          return ret;
        }
      }
    }
  }
  addToPath(path4, added, removed, oldPosInc, options) {
    const last = path4.lastComponent;
    if (last && !options.oneChangePerToken && last.added === added && last.removed === removed) {
      return {
        oldPos: path4.oldPos + oldPosInc,
        lastComponent: { count: last.count + 1, added, removed, previousComponent: last.previousComponent }
      };
    } else {
      return {
        oldPos: path4.oldPos + oldPosInc,
        lastComponent: { count: 1, added, removed, previousComponent: last }
      };
    }
  }
  extractCommon(basePath, newTokens, oldTokens, diagonalPath, options) {
    const newLen = newTokens.length, oldLen = oldTokens.length;
    let oldPos = basePath.oldPos, newPos = oldPos - diagonalPath, commonCount = 0;
    while (newPos + 1 < newLen && oldPos + 1 < oldLen && this.equals(oldTokens[oldPos + 1], newTokens[newPos + 1], options)) {
      newPos++;
      oldPos++;
      commonCount++;
      if (options.oneChangePerToken) {
        basePath.lastComponent = { count: 1, previousComponent: basePath.lastComponent, added: false, removed: false };
      }
    }
    if (commonCount && !options.oneChangePerToken) {
      basePath.lastComponent = { count: commonCount, previousComponent: basePath.lastComponent, added: false, removed: false };
    }
    basePath.oldPos = oldPos;
    return newPos;
  }
  equals(left, right, options) {
    if (options.comparator) {
      return options.comparator(left, right);
    } else {
      return left === right || !!options.ignoreCase && left.toLowerCase() === right.toLowerCase();
    }
  }
  removeEmpty(array) {
    const ret = [];
    for (let i = 0; i < array.length; i++) {
      if (array[i]) {
        ret.push(array[i]);
      }
    }
    return ret;
  }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  castInput(value, options) {
    return value;
  }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  tokenize(value, options) {
    return Array.from(value);
  }
  join(chars) {
    return chars.join("");
  }
  postProcess(changeObjects, options) {
    return changeObjects;
  }
  get useLongestToken() {
    return false;
  }
  buildValues(lastComponent, newTokens, oldTokens) {
    const components = [];
    let nextComponent;
    while (lastComponent) {
      components.push(lastComponent);
      nextComponent = lastComponent.previousComponent;
      delete lastComponent.previousComponent;
      lastComponent = nextComponent;
    }
    components.reverse();
    const componentLen = components.length;
    let componentPos = 0, newPos = 0, oldPos = 0;
    for (; componentPos < componentLen; componentPos++) {
      const component = components[componentPos];
      if (!component.removed) {
        if (!component.added && this.useLongestToken) {
          let value = newTokens.slice(newPos, newPos + component.count);
          value = value.map(function(value2, i) {
            const oldValue = oldTokens[oldPos + i];
            return oldValue.length > value2.length ? oldValue : value2;
          });
          component.value = this.join(value);
        } else {
          component.value = this.join(newTokens.slice(newPos, newPos + component.count));
        }
        newPos += component.count;
        if (!component.added) {
          oldPos += component.count;
        }
      } else {
        component.value = this.join(oldTokens.slice(oldPos, oldPos + component.count));
        oldPos += component.count;
      }
    }
    return components;
  }
};

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/diff/libesm/util/string.js
function longestCommonPrefix(str1, str2) {
  let i;
  for (i = 0; i < str1.length && i < str2.length; i++) {
    if (str1[i] != str2[i]) {
      return str1.slice(0, i);
    }
  }
  return str1.slice(0, i);
}
function longestCommonSuffix(str1, str2) {
  let i;
  if (!str1 || !str2 || str1[str1.length - 1] != str2[str2.length - 1]) {
    return "";
  }
  for (i = 0; i < str1.length && i < str2.length; i++) {
    if (str1[str1.length - (i + 1)] != str2[str2.length - (i + 1)]) {
      return str1.slice(-i);
    }
  }
  return str1.slice(-i);
}
function replacePrefix(string, oldPrefix, newPrefix) {
  if (string.slice(0, oldPrefix.length) != oldPrefix) {
    throw Error(`string ${JSON.stringify(string)} doesn't start with prefix ${JSON.stringify(oldPrefix)}; this is a bug`);
  }
  return newPrefix + string.slice(oldPrefix.length);
}
function replaceSuffix(string, oldSuffix, newSuffix) {
  if (!oldSuffix) {
    return string + newSuffix;
  }
  if (string.slice(-oldSuffix.length) != oldSuffix) {
    throw Error(`string ${JSON.stringify(string)} doesn't end with suffix ${JSON.stringify(oldSuffix)}; this is a bug`);
  }
  return string.slice(0, -oldSuffix.length) + newSuffix;
}
function removePrefix(string, oldPrefix) {
  return replacePrefix(string, oldPrefix, "");
}
function removeSuffix(string, oldSuffix) {
  return replaceSuffix(string, oldSuffix, "");
}
function maximumOverlap(string1, string2) {
  return string2.slice(0, overlapCount(string1, string2));
}
function overlapCount(a, b) {
  let startA = 0;
  if (a.length > b.length) {
    startA = a.length - b.length;
  }
  let endB = b.length;
  if (a.length < b.length) {
    endB = a.length;
  }
  const map = Array(endB);
  let k = 0;
  map[0] = 0;
  for (let j = 1; j < endB; j++) {
    if (b[j] == b[k]) {
      map[j] = map[k];
    } else {
      map[j] = k;
    }
    while (k > 0 && b[j] != b[k]) {
      k = map[k];
    }
    if (b[j] == b[k]) {
      k++;
    }
  }
  k = 0;
  for (let i = startA; i < a.length; i++) {
    while (k > 0 && a[i] != b[k]) {
      k = map[k];
    }
    if (a[i] == b[k]) {
      k++;
    }
  }
  return k;
}
function segment(string, segmenter) {
  const parts = [];
  for (const segmentObj of Array.from(segmenter.segment(string))) {
    const segment2 = segmentObj.segment;
    if (parts.length && /\s/.test(parts[parts.length - 1]) && /\s/.test(segment2)) {
      parts[parts.length - 1] += segment2;
    } else {
      parts.push(segment2);
    }
  }
  return parts;
}
function trailingWs(string, segmenter) {
  if (segmenter) {
    return leadingAndTrailingWs(string, segmenter)[1];
  }
  let i;
  for (i = string.length - 1; i >= 0; i--) {
    if (!string[i].match(/\s/)) {
      break;
    }
  }
  return string.substring(i + 1);
}
function leadingWs(string, segmenter) {
  if (segmenter) {
    return leadingAndTrailingWs(string, segmenter)[0];
  }
  const match2 = string.match(/^\s*/);
  return match2 ? match2[0] : "";
}
function leadingAndTrailingWs(string, segmenter) {
  if (!segmenter) {
    return [leadingWs(string), trailingWs(string)];
  }
  if (segmenter.resolvedOptions().granularity != "word") {
    throw new Error('The segmenter passed must have a granularity of "word"');
  }
  const segments = segment(string, segmenter);
  const firstSeg = segments[0];
  const lastSeg = segments[segments.length - 1];
  const head = /\s/.test(firstSeg) ? firstSeg : "";
  const tail = /\s/.test(lastSeg) ? lastSeg : "";
  return [head, tail];
}

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/diff/libesm/diff/word.js
var extendedWordChars = "a-zA-Z0-9_\\u{AD}\\u{C0}-\\u{D6}\\u{D8}-\\u{F6}\\u{F8}-\\u{2C6}\\u{2C8}-\\u{2D7}\\u{2DE}-\\u{2FF}\\u{1E00}-\\u{1EFF}";
var tokenizeIncludingWhitespace = new RegExp(`[${extendedWordChars}]+|\\s+|[^${extendedWordChars}]`, "ug");
var WordDiff = class extends Diff {
  equals(left, right, options) {
    if (options.ignoreCase) {
      left = left.toLowerCase();
      right = right.toLowerCase();
    }
    return left.trim() === right.trim();
  }
  tokenize(value, options = {}) {
    let parts;
    if (options.intlSegmenter) {
      const segmenter = options.intlSegmenter;
      if (segmenter.resolvedOptions().granularity != "word") {
        throw new Error('The segmenter passed must have a granularity of "word"');
      }
      parts = segment(value, segmenter);
    } else {
      parts = value.match(tokenizeIncludingWhitespace) || [];
    }
    const tokens = [];
    let prevPart = null;
    parts.forEach((part) => {
      if (/\s/.test(part)) {
        if (prevPart == null) {
          tokens.push(part);
        } else {
          tokens.push(tokens.pop() + part);
        }
      } else if (prevPart != null && /\s/.test(prevPart)) {
        if (tokens[tokens.length - 1] == prevPart) {
          tokens.push(tokens.pop() + part);
        } else {
          tokens.push(prevPart + part);
        }
      } else {
        tokens.push(part);
      }
      prevPart = part;
    });
    return tokens;
  }
  join(tokens) {
    return tokens.map((token, i) => {
      if (i == 0) {
        return token;
      } else {
        return token.replace(/^\s+/, "");
      }
    }).join("");
  }
  postProcess(changes, options) {
    if (!changes || options.oneChangePerToken) {
      return changes;
    }
    let lastKeep = null;
    let insertion = null;
    let deletion = null;
    changes.forEach((change) => {
      if (change.added) {
        insertion = change;
      } else if (change.removed) {
        deletion = change;
      } else {
        if (insertion || deletion) {
          dedupeWhitespaceInChangeObjects(lastKeep, deletion, insertion, change, options.intlSegmenter);
        }
        lastKeep = change;
        insertion = null;
        deletion = null;
      }
    });
    if (insertion || deletion) {
      dedupeWhitespaceInChangeObjects(lastKeep, deletion, insertion, null, options.intlSegmenter);
    }
    return changes;
  }
};
var wordDiff = new WordDiff();
function diffWords(oldStr, newStr, options) {
  if ((options === null || options === void 0 ? void 0 : options.ignoreWhitespace) != null && !options.ignoreWhitespace) {
    return diffWordsWithSpace(oldStr, newStr, options);
  }
  return wordDiff.diff(oldStr, newStr, options);
}
function dedupeWhitespaceInChangeObjects(startKeep, deletion, insertion, endKeep, segmenter) {
  if (deletion && insertion) {
    const [oldWsPrefix, oldWsSuffix] = leadingAndTrailingWs(deletion.value, segmenter);
    const [newWsPrefix, newWsSuffix] = leadingAndTrailingWs(insertion.value, segmenter);
    if (startKeep) {
      const commonWsPrefix = longestCommonPrefix(oldWsPrefix, newWsPrefix);
      startKeep.value = replaceSuffix(startKeep.value, newWsPrefix, commonWsPrefix);
      deletion.value = removePrefix(deletion.value, commonWsPrefix);
      insertion.value = removePrefix(insertion.value, commonWsPrefix);
    }
    if (endKeep) {
      const commonWsSuffix = longestCommonSuffix(oldWsSuffix, newWsSuffix);
      endKeep.value = replacePrefix(endKeep.value, newWsSuffix, commonWsSuffix);
      deletion.value = removeSuffix(deletion.value, commonWsSuffix);
      insertion.value = removeSuffix(insertion.value, commonWsSuffix);
    }
  } else if (insertion) {
    if (startKeep) {
      const ws = leadingWs(insertion.value, segmenter);
      insertion.value = insertion.value.substring(ws.length);
    }
    if (endKeep) {
      const ws = leadingWs(endKeep.value, segmenter);
      endKeep.value = endKeep.value.substring(ws.length);
    }
  } else if (startKeep && endKeep) {
    const newWsFull = leadingWs(endKeep.value, segmenter), [delWsStart, delWsEnd] = leadingAndTrailingWs(deletion.value, segmenter);
    const newWsStart = longestCommonPrefix(newWsFull, delWsStart);
    deletion.value = removePrefix(deletion.value, newWsStart);
    const newWsEnd = longestCommonSuffix(removePrefix(newWsFull, newWsStart), delWsEnd);
    deletion.value = removeSuffix(deletion.value, newWsEnd);
    endKeep.value = replacePrefix(endKeep.value, newWsFull, newWsEnd);
    startKeep.value = replaceSuffix(startKeep.value, newWsFull, newWsFull.slice(0, newWsFull.length - newWsEnd.length));
  } else if (endKeep) {
    const endKeepWsPrefix = leadingWs(endKeep.value, segmenter);
    const deletionWsSuffix = trailingWs(deletion.value, segmenter);
    const overlap = maximumOverlap(deletionWsSuffix, endKeepWsPrefix);
    deletion.value = removeSuffix(deletion.value, overlap);
  } else if (startKeep) {
    const startKeepWsSuffix = trailingWs(startKeep.value, segmenter);
    const deletionWsPrefix = leadingWs(deletion.value, segmenter);
    const overlap = maximumOverlap(startKeepWsSuffix, deletionWsPrefix);
    deletion.value = removePrefix(deletion.value, overlap);
  }
}
var WordsWithSpaceDiff = class extends Diff {
  tokenize(value) {
    const regex2 = new RegExp(`(\\r?\\n)|[${extendedWordChars}]+|[^\\S\\n\\r]+|[^${extendedWordChars}]`, "ug");
    return value.match(regex2) || [];
  }
};
var wordsWithSpaceDiff = new WordsWithSpaceDiff();
function diffWordsWithSpace(oldStr, newStr, options) {
  return wordsWithSpaceDiff.diff(oldStr, newStr, options);
}

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/diff/libesm/diff/line.js
var LineDiff = class extends Diff {
  constructor() {
    super(...arguments);
    this.tokenize = tokenize;
  }
  equals(left, right, options) {
    if (options.ignoreWhitespace) {
      if (!options.newlineIsToken || !left.includes("\n")) {
        left = left.trim();
      }
      if (!options.newlineIsToken || !right.includes("\n")) {
        right = right.trim();
      }
    } else if (options.ignoreNewlineAtEof && !options.newlineIsToken) {
      if (left.endsWith("\n")) {
        left = left.slice(0, -1);
      }
      if (right.endsWith("\n")) {
        right = right.slice(0, -1);
      }
    }
    return super.equals(left, right, options);
  }
};
var lineDiff = new LineDiff();
function diffLines(oldStr, newStr, options) {
  return lineDiff.diff(oldStr, newStr, options);
}
function tokenize(value, options) {
  if (options.stripTrailingCr) {
    value = value.replace(/\r\n/g, "\n");
  }
  const retLines = [], linesAndNewlines = value.split(/(\n|\r\n)/);
  if (!linesAndNewlines[linesAndNewlines.length - 1]) {
    linesAndNewlines.pop();
  }
  for (let i = 0; i < linesAndNewlines.length; i++) {
    const line = linesAndNewlines[i];
    if (i % 2 && !options.newlineIsToken) {
      retLines[retLines.length - 1] += line;
    } else {
      retLines.push(line);
    }
  }
  return retLines;
}

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/diff/libesm/patch/create.js
var INCLUDE_HEADERS = {
  includeIndex: true,
  includeUnderline: true,
  includeFileHeaders: true
};
var FILE_HEADERS_ONLY = {
  includeIndex: false,
  includeUnderline: false,
  includeFileHeaders: true
};
function structuredPatch(oldFileName, newFileName, oldStr, newStr, oldHeader, newHeader, options) {
  let optionsObj;
  if (!options) {
    optionsObj = {};
  } else if (typeof options === "function") {
    optionsObj = { callback: options };
  } else {
    optionsObj = options;
  }
  if (typeof optionsObj.context === "undefined") {
    optionsObj.context = 4;
  }
  const context = optionsObj.context;
  if (optionsObj.newlineIsToken) {
    throw new Error("newlineIsToken may not be used with patch-generation functions, only with diffing functions");
  }
  if (!optionsObj.callback) {
    return diffLinesResultToPatch(diffLines(oldStr, newStr, optionsObj));
  } else {
    const { callback } = optionsObj;
    diffLines(oldStr, newStr, Object.assign(Object.assign({}, optionsObj), { callback: (diff) => {
      const patch = diffLinesResultToPatch(diff);
      callback(patch);
    } }));
  }
  function diffLinesResultToPatch(diff) {
    if (!diff) {
      return;
    }
    diff.push({ value: "", lines: [] });
    function contextLines(lines) {
      return lines.map(function(entry) {
        return " " + entry;
      });
    }
    const hunks = [];
    let oldRangeStart = 0, newRangeStart = 0, curRange = [], oldLine = 1, newLine = 1;
    for (let i = 0; i < diff.length; i++) {
      const current = diff[i], lines = current.lines || splitLines(current.value);
      current.lines = lines;
      if (current.added || current.removed) {
        if (!oldRangeStart) {
          const prev = diff[i - 1];
          oldRangeStart = oldLine;
          newRangeStart = newLine;
          if (prev) {
            curRange = context > 0 ? contextLines(prev.lines.slice(-context)) : [];
            oldRangeStart -= curRange.length;
            newRangeStart -= curRange.length;
          }
        }
        for (const line of lines) {
          curRange.push((current.added ? "+" : "-") + line);
        }
        if (current.added) {
          newLine += lines.length;
        } else {
          oldLine += lines.length;
        }
      } else {
        if (oldRangeStart) {
          if (lines.length <= context * 2 && i < diff.length - 2) {
            for (const line of contextLines(lines)) {
              curRange.push(line);
            }
          } else {
            const contextSize = Math.min(lines.length, context);
            for (const line of contextLines(lines.slice(0, contextSize))) {
              curRange.push(line);
            }
            const hunk = {
              oldStart: oldRangeStart,
              oldLines: oldLine - oldRangeStart + contextSize,
              newStart: newRangeStart,
              newLines: newLine - newRangeStart + contextSize,
              lines: curRange
            };
            hunks.push(hunk);
            oldRangeStart = 0;
            newRangeStart = 0;
            curRange = [];
          }
        }
        oldLine += lines.length;
        newLine += lines.length;
      }
    }
    for (const hunk of hunks) {
      for (let i = 0; i < hunk.lines.length; i++) {
        if (hunk.lines[i].endsWith("\n")) {
          hunk.lines[i] = hunk.lines[i].slice(0, -1);
        } else {
          hunk.lines.splice(i + 1, 0, "\\ No newline at end of file");
          i++;
        }
      }
    }
    return {
      oldFileName,
      newFileName,
      oldHeader,
      newHeader,
      hunks
    };
  }
}
function formatPatch(patch, headerOptions) {
  if (!headerOptions) {
    headerOptions = INCLUDE_HEADERS;
  }
  if (Array.isArray(patch)) {
    if (patch.length > 1 && !headerOptions.includeFileHeaders) {
      throw new Error("Cannot omit file headers on a multi-file patch. (The result would be unparseable; how would a tool trying to apply the patch know which changes are to which file?)");
    }
    return patch.map((p) => formatPatch(p, headerOptions)).join("\n");
  }
  const ret = [];
  if (headerOptions.includeIndex && patch.oldFileName == patch.newFileName) {
    ret.push("Index: " + patch.oldFileName);
  }
  if (headerOptions.includeUnderline) {
    ret.push("===================================================================");
  }
  if (headerOptions.includeFileHeaders) {
    ret.push("--- " + patch.oldFileName + (typeof patch.oldHeader === "undefined" ? "" : "	" + patch.oldHeader));
    ret.push("+++ " + patch.newFileName + (typeof patch.newHeader === "undefined" ? "" : "	" + patch.newHeader));
  }
  for (let i = 0; i < patch.hunks.length; i++) {
    const hunk = patch.hunks[i];
    if (hunk.oldLines === 0) {
      hunk.oldStart -= 1;
    }
    if (hunk.newLines === 0) {
      hunk.newStart -= 1;
    }
    ret.push("@@ -" + hunk.oldStart + "," + hunk.oldLines + " +" + hunk.newStart + "," + hunk.newLines + " @@");
    for (const line of hunk.lines) {
      ret.push(line);
    }
  }
  return ret.join("\n") + "\n";
}
function createTwoFilesPatch(oldFileName, newFileName, oldStr, newStr, oldHeader, newHeader, options) {
  if (typeof options === "function") {
    options = { callback: options };
  }
  if (!(options === null || options === void 0 ? void 0 : options.callback)) {
    const patchObj = structuredPatch(oldFileName, newFileName, oldStr, newStr, oldHeader, newHeader, options);
    if (!patchObj) {
      return;
    }
    return formatPatch(patchObj, options === null || options === void 0 ? void 0 : options.headerOptions);
  } else {
    const { callback } = options;
    structuredPatch(oldFileName, newFileName, oldStr, newStr, oldHeader, newHeader, Object.assign(Object.assign({}, options), { callback: (patchObj) => {
      if (!patchObj) {
        callback(void 0);
      } else {
        callback(formatPatch(patchObj, options.headerOptions));
      }
    } }));
  }
}
function splitLines(text) {
  const hasTrailingNl = text.endsWith("\n");
  const result = text.split("\n").map((line) => line + "\n");
  if (hasTrailingNl) {
    result.pop();
  } else {
    result.push(result.pop().slice(0, -1));
  }
  return result;
}

// target/vendor-pi/dist/core/tools/edit-diff.js
import { constants as constants2 } from "fs";
import { access as access2, readFile } from "fs/promises";

// target/vendor-pi/dist/utils/text.js
function splitBom(content) {
  return content.startsWith("\uFEFF") ? { bom: "\uFEFF", text: content.slice(1) } : { bom: "", text: content };
}
function stripBom(content) {
  return splitBom(content).text;
}

// target/vendor-pi/dist/core/tools/path-utils.js
import { accessSync, constants } from "node:fs";
import { access } from "node:fs/promises";
var NARROW_NO_BREAK_SPACE = "\u202F";
function tryMacOSScreenshotPath(filePath) {
  return filePath.replace(/ (AM|PM)\./gi, `${NARROW_NO_BREAK_SPACE}$1.`);
}
function tryNFDVariant(filePath) {
  return filePath.normalize("NFD");
}
function tryCurlyQuoteVariant(filePath) {
  return filePath.replace(/'/g, "\u2019");
}
async function pathExists(filePath) {
  try {
    await access(filePath, constants.F_OK);
    return true;
  } catch {
    return false;
  }
}
function resolveToCwd(filePath, cwd) {
  return resolvePath(filePath, cwd, { normalizeUnicodeSpaces: true, stripAtPrefix: true });
}
async function resolveReadPathAsync(filePath, cwd) {
  const resolved = resolveToCwd(filePath, cwd);
  if (await pathExists(resolved)) {
    return resolved;
  }
  const amPmVariant = tryMacOSScreenshotPath(resolved);
  if (amPmVariant !== resolved && await pathExists(amPmVariant)) {
    return amPmVariant;
  }
  const nfdVariant = tryNFDVariant(resolved);
  if (nfdVariant !== resolved && await pathExists(nfdVariant)) {
    return nfdVariant;
  }
  const curlyVariant = tryCurlyQuoteVariant(resolved);
  if (curlyVariant !== resolved && await pathExists(curlyVariant)) {
    return curlyVariant;
  }
  const nfdCurlyVariant = tryCurlyQuoteVariant(nfdVariant);
  if (nfdCurlyVariant !== resolved && await pathExists(nfdCurlyVariant)) {
    return nfdCurlyVariant;
  }
  return resolved;
}

// target/vendor-pi/dist/core/tools/edit-diff.js
function detectLineEnding(content) {
  const crlfIdx = content.indexOf("\r\n");
  const lfIdx = content.indexOf("\n");
  if (lfIdx === -1)
    return "\n";
  if (crlfIdx === -1)
    return "\n";
  return crlfIdx < lfIdx ? "\r\n" : "\n";
}
function normalizeToLF(text) {
  return text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
}
function restoreLineEndings(text, ending) {
  return ending === "\r\n" ? text.replace(/\n/g, "\r\n") : text;
}
function normalizeForFuzzyMatch(text) {
  return text.normalize("NFKC").split("\n").map((line) => line.trimEnd()).join("\n").replace(/[\u2018\u2019\u201A\u201B]/g, "'").replace(/[\u201C\u201D\u201E\u201F]/g, '"').replace(/[\u2010\u2011\u2012\u2013\u2014\u2015\u2212]/g, "-").replace(/[\u00A0\u2002-\u200A\u202F\u205F\u3000]/g, " ");
}
function splitLinesWithEndings(content) {
  return content.match(/[^\n]*\n|[^\n]+/g) ?? [];
}
function getLineSpans(content) {
  let offset = 0;
  return splitLinesWithEndings(content).map((line) => {
    const span = { start: offset, end: offset + line.length };
    offset = span.end;
    return span;
  });
}
function getReplacementLineRange(lines, replacement) {
  const replacementStart = replacement.matchIndex;
  const replacementEnd = replacement.matchIndex + replacement.matchLength;
  let startLine = -1;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (replacementStart >= line.start && replacementStart < line.end) {
      startLine = i;
      break;
    }
  }
  if (startLine === -1) {
    throw new Error("Replacement range is outside the base content.");
  }
  let endLine = startLine;
  while (endLine < lines.length && lines[endLine].end < replacementEnd) {
    endLine++;
  }
  if (endLine >= lines.length) {
    throw new Error("Replacement range is outside the base content.");
  }
  return { startLine, endLine: endLine + 1 };
}
function applyReplacements(content, replacements, offset = 0) {
  let result = content;
  for (let i = replacements.length - 1; i >= 0; i--) {
    const replacement = replacements[i];
    const matchIndex = replacement.matchIndex - offset;
    result = result.substring(0, matchIndex) + replacement.newText + result.substring(matchIndex + replacement.matchLength);
  }
  return result;
}
function applyReplacementsPreservingUnchangedLines(originalContent, baseContent, replacements) {
  const originalLines = splitLinesWithEndings(originalContent);
  const baseLines = getLineSpans(baseContent);
  if (originalLines.length !== baseLines.length) {
    throw new Error("Cannot preserve unchanged lines because the base content has a different line count.");
  }
  const groups = [];
  const sortedReplacements = [...replacements].sort((a, b) => a.matchIndex - b.matchIndex);
  for (const replacement of sortedReplacements) {
    const range2 = getReplacementLineRange(baseLines, replacement);
    const current = groups[groups.length - 1];
    if (current && range2.startLine < current.endLine) {
      current.endLine = Math.max(current.endLine, range2.endLine);
      current.replacements.push(replacement);
      continue;
    }
    groups.push({ ...range2, replacements: [replacement] });
  }
  let originalLineIndex = 0;
  let result = "";
  for (const group of groups) {
    result += originalLines.slice(originalLineIndex, group.startLine).join("");
    const groupStartOffset = baseLines[group.startLine].start;
    const groupEndOffset = baseLines[group.endLine - 1].end;
    result += applyReplacements(baseContent.slice(groupStartOffset, groupEndOffset), group.replacements, groupStartOffset);
    originalLineIndex = group.endLine;
  }
  result += originalLines.slice(originalLineIndex).join("");
  return result;
}
function fuzzyFindText(content, oldText) {
  const exactIndex = content.indexOf(oldText);
  if (exactIndex !== -1) {
    return {
      found: true,
      index: exactIndex,
      matchLength: oldText.length,
      usedFuzzyMatch: false,
      contentForReplacement: content
    };
  }
  const fuzzyContent = normalizeForFuzzyMatch(content);
  const fuzzyOldText = normalizeForFuzzyMatch(oldText);
  const fuzzyIndex = fuzzyContent.indexOf(fuzzyOldText);
  if (fuzzyIndex === -1) {
    return {
      found: false,
      index: -1,
      matchLength: 0,
      usedFuzzyMatch: false,
      contentForReplacement: content
    };
  }
  return {
    found: true,
    index: fuzzyIndex,
    matchLength: fuzzyOldText.length,
    usedFuzzyMatch: true,
    contentForReplacement: fuzzyContent
  };
}
function countOccurrences(content, oldText) {
  const fuzzyContent = normalizeForFuzzyMatch(content);
  const fuzzyOldText = normalizeForFuzzyMatch(oldText);
  return fuzzyContent.split(fuzzyOldText).length - 1;
}
function getNotFoundError(path4, editIndex, totalEdits) {
  if (totalEdits === 1) {
    return new Error(`Could not find the exact text in ${path4}. The old text must match exactly including all whitespace and newlines.`);
  }
  return new Error(`Could not find edits[${editIndex}] in ${path4}. The oldText must match exactly including all whitespace and newlines.`);
}
function getDuplicateError(path4, editIndex, totalEdits, occurrences) {
  if (totalEdits === 1) {
    return new Error(`Found ${occurrences} occurrences of the text in ${path4}. The text must be unique. Please provide more context to make it unique.`);
  }
  return new Error(`Found ${occurrences} occurrences of edits[${editIndex}] in ${path4}. Each oldText must be unique. Please provide more context to make it unique.`);
}
function getEmptyOldTextError(path4, editIndex, totalEdits) {
  if (totalEdits === 1) {
    return new Error(`oldText must not be empty in ${path4}.`);
  }
  return new Error(`edits[${editIndex}].oldText must not be empty in ${path4}.`);
}
function getNoChangeError(path4, totalEdits) {
  if (totalEdits === 1) {
    return new Error(`No changes made to ${path4}. The replacement produced identical content. This might indicate an issue with special characters or the text not existing as expected.`);
  }
  return new Error(`No changes made to ${path4}. The replacements produced identical content.`);
}
function applyEditsToNormalizedContent(normalizedContent, edits, path4) {
  const normalizedEdits = edits.map((edit) => ({
    oldText: normalizeToLF(edit.oldText),
    newText: normalizeToLF(edit.newText)
  }));
  for (let i = 0; i < normalizedEdits.length; i++) {
    if (normalizedEdits[i].oldText.length === 0) {
      throw getEmptyOldTextError(path4, i, normalizedEdits.length);
    }
  }
  const initialMatches = normalizedEdits.map((edit) => fuzzyFindText(normalizedContent, edit.oldText));
  const usedFuzzyMatch = initialMatches.some((match2) => match2.usedFuzzyMatch);
  const replacementBaseContent = usedFuzzyMatch ? normalizeForFuzzyMatch(normalizedContent) : normalizedContent;
  const matchedEdits = [];
  for (let i = 0; i < normalizedEdits.length; i++) {
    const edit = normalizedEdits[i];
    const matchResult = fuzzyFindText(replacementBaseContent, edit.oldText);
    if (!matchResult.found) {
      throw getNotFoundError(path4, i, normalizedEdits.length);
    }
    const occurrences = countOccurrences(replacementBaseContent, edit.oldText);
    if (occurrences > 1) {
      throw getDuplicateError(path4, i, normalizedEdits.length, occurrences);
    }
    matchedEdits.push({
      editIndex: i,
      matchIndex: matchResult.index,
      matchLength: matchResult.matchLength,
      newText: edit.newText
    });
  }
  matchedEdits.sort((a, b) => a.matchIndex - b.matchIndex);
  for (let i = 1; i < matchedEdits.length; i++) {
    const previous = matchedEdits[i - 1];
    const current = matchedEdits[i];
    if (previous.matchIndex + previous.matchLength > current.matchIndex) {
      throw new Error(`edits[${previous.editIndex}] and edits[${current.editIndex}] overlap in ${path4}. Merge them into one edit or target disjoint regions.`);
    }
  }
  const baseContent = normalizedContent;
  const newContent = usedFuzzyMatch ? applyReplacementsPreservingUnchangedLines(normalizedContent, replacementBaseContent, matchedEdits) : applyReplacements(replacementBaseContent, matchedEdits);
  if (baseContent === newContent) {
    throw getNoChangeError(path4, normalizedEdits.length);
  }
  return { baseContent, newContent };
}
function generateUnifiedPatch(path4, oldContent, newContent, contextLines = 4) {
  return createTwoFilesPatch(path4, path4, oldContent, newContent, void 0, void 0, {
    context: contextLines,
    headerOptions: FILE_HEADERS_ONLY
  });
}
function generateDiffString(oldContent, newContent, contextLines = 4) {
  const parts = diffLines(oldContent, newContent);
  const output = [];
  const oldLines = oldContent.split("\n");
  const newLines = newContent.split("\n");
  const maxLineNum = Math.max(oldLines.length, newLines.length);
  const lineNumWidth = String(maxLineNum).length;
  let oldLineNum = 1;
  let newLineNum = 1;
  let lastWasChange = false;
  let firstChangedLine;
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i];
    const raw = part.value.split("\n");
    if (raw[raw.length - 1] === "") {
      raw.pop();
    }
    if (part.added || part.removed) {
      if (firstChangedLine === void 0) {
        firstChangedLine = newLineNum;
      }
      for (const line of raw) {
        if (part.added) {
          const lineNum = String(newLineNum).padStart(lineNumWidth, " ");
          output.push(`+${lineNum} ${line}`);
          newLineNum++;
        } else {
          const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
          output.push(`-${lineNum} ${line}`);
          oldLineNum++;
        }
      }
      lastWasChange = true;
    } else {
      const nextPartIsChange = i < parts.length - 1 && (parts[i + 1].added || parts[i + 1].removed);
      const hasLeadingChange = lastWasChange;
      const hasTrailingChange = nextPartIsChange;
      if (hasLeadingChange && hasTrailingChange) {
        if (raw.length <= contextLines * 2) {
          for (const line of raw) {
            const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
            output.push(` ${lineNum} ${line}`);
            oldLineNum++;
            newLineNum++;
          }
        } else {
          const leadingLines = raw.slice(0, contextLines);
          const trailingLines = raw.slice(raw.length - contextLines);
          const skippedLines = raw.length - leadingLines.length - trailingLines.length;
          for (const line of leadingLines) {
            const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
            output.push(` ${lineNum} ${line}`);
            oldLineNum++;
            newLineNum++;
          }
          output.push(` ${"".padStart(lineNumWidth, " ")} ...`);
          oldLineNum += skippedLines;
          newLineNum += skippedLines;
          for (const line of trailingLines) {
            const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
            output.push(` ${lineNum} ${line}`);
            oldLineNum++;
            newLineNum++;
          }
        }
      } else if (hasLeadingChange) {
        const shownLines = raw.slice(0, contextLines);
        const skippedLines = raw.length - shownLines.length;
        for (const line of shownLines) {
          const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
          output.push(` ${lineNum} ${line}`);
          oldLineNum++;
          newLineNum++;
        }
        if (skippedLines > 0) {
          output.push(` ${"".padStart(lineNumWidth, " ")} ...`);
          oldLineNum += skippedLines;
          newLineNum += skippedLines;
        }
      } else if (hasTrailingChange) {
        const skippedLines = Math.max(0, raw.length - contextLines);
        if (skippedLines > 0) {
          output.push(` ${"".padStart(lineNumWidth, " ")} ...`);
          oldLineNum += skippedLines;
          newLineNum += skippedLines;
        }
        for (const line of raw.slice(skippedLines)) {
          const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
          output.push(` ${lineNum} ${line}`);
          oldLineNum++;
          newLineNum++;
        }
      } else {
        oldLineNum += raw.length;
        newLineNum += raw.length;
      }
      lastWasChange = false;
    }
  }
  return { diff: output.join("\n"), firstChangedLine };
}
async function computeEditsDiff(path4, edits, cwd) {
  const absolutePath = resolveToCwd(path4, cwd);
  try {
    try {
      await access2(absolutePath, constants2.R_OK);
    } catch (error) {
      const errorMessage = error instanceof Error && "code" in error ? `Error code: ${error.code}` : String(error);
      return { error: `Could not edit file: ${path4}. ${errorMessage}.` };
    }
    const rawContent = await readFile(absolutePath, "utf-8");
    const { text: content } = splitBom(rawContent);
    const normalizedContent = normalizeToLF(content);
    const { baseContent, newContent } = applyEditsToNormalizedContent(normalizedContent, edits, path4);
    return generateDiffString(baseContent, newContent);
  } catch (err) {
    return { error: err instanceof Error ? err.message : String(err) };
  }
}

// target/vendor-pi/dist/core/model-resolver.js
import { modelsAreEqual } from "@earendil-works/pi-ai";

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/balanced-match/dist/esm/index.js
var balanced = (a, b, str2) => {
  const ma = a instanceof RegExp ? maybeMatch(a, str2) : a;
  const mb = b instanceof RegExp ? maybeMatch(b, str2) : b;
  const r = ma !== null && mb != null && range(ma, mb, str2);
  return r && {
    start: r[0],
    end: r[1],
    pre: str2.slice(0, r[0]),
    body: str2.slice(r[0] + ma.length, r[1]),
    post: str2.slice(r[1] + mb.length)
  };
};
var maybeMatch = (reg, str2) => {
  const m = str2.match(reg);
  return m ? m[0] : null;
};
var range = (a, b, str2) => {
  let begs, beg, left, right = void 0, result;
  let ai = str2.indexOf(a);
  let bi = str2.indexOf(b, ai + 1);
  let i = ai;
  if (ai >= 0 && bi > 0) {
    if (a === b) {
      return [ai, bi];
    }
    begs = [];
    left = str2.length;
    while (i >= 0 && !result) {
      if (i === ai) {
        begs.push(i);
        ai = str2.indexOf(a, i + 1);
      } else if (begs.length === 1) {
        const r = begs.pop();
        if (r !== void 0)
          result = [r, bi];
      } else {
        beg = begs.pop();
        if (beg !== void 0 && beg < left) {
          left = beg;
          right = bi;
        }
        bi = str2.indexOf(b, i + 1);
      }
      i = ai < bi && ai >= 0 ? ai : bi;
    }
    if (begs.length && right !== void 0) {
      result = [left, right];
    }
  }
  return result;
};

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/brace-expansion/dist/esm/index.js
var escSlash = "\0SLASH" + Math.random() + "\0";
var escOpen = "\0OPEN" + Math.random() + "\0";
var escClose = "\0CLOSE" + Math.random() + "\0";
var escComma = "\0COMMA" + Math.random() + "\0";
var escPeriod = "\0PERIOD" + Math.random() + "\0";
var escSlashPattern = new RegExp(escSlash, "g");
var escOpenPattern = new RegExp(escOpen, "g");
var escClosePattern = new RegExp(escClose, "g");
var escCommaPattern = new RegExp(escComma, "g");
var escPeriodPattern = new RegExp(escPeriod, "g");
var slashPattern = /\\\\/g;
var openPattern = /\\{/g;
var closePattern = /\\}/g;
var commaPattern = /\\,/g;
var periodPattern = /\\\./g;
var EXPANSION_MAX = 1e5;
var EXPANSION_MAX_LENGTH = 4e6;
function numeric(str2) {
  return !isNaN(str2) ? parseInt(str2, 10) : str2.charCodeAt(0);
}
function escapeBraces(str2) {
  return str2.replace(slashPattern, escSlash).replace(openPattern, escOpen).replace(closePattern, escClose).replace(commaPattern, escComma).replace(periodPattern, escPeriod);
}
function unescapeBraces(str2) {
  return str2.replace(escSlashPattern, "\\").replace(escOpenPattern, "{").replace(escClosePattern, "}").replace(escCommaPattern, ",").replace(escPeriodPattern, ".");
}
function parseCommaParts(str2) {
  if (!str2) {
    return [""];
  }
  const parts = [];
  const m = balanced("{", "}", str2);
  if (!m) {
    return str2.split(",");
  }
  const { pre, body, post } = m;
  const p = pre.split(",");
  p[p.length - 1] += "{" + body + "}";
  const postParts = parseCommaParts(post);
  if (post.length) {
    ;
    p[p.length - 1] += postParts.shift();
    p.push.apply(p, postParts);
  }
  parts.push.apply(parts, p);
  return parts;
}
function expand(str2, options = {}) {
  if (!str2) {
    return [];
  }
  const { max = EXPANSION_MAX, maxLength = EXPANSION_MAX_LENGTH } = options;
  if (str2.slice(0, 2) === "{}") {
    str2 = "\\{\\}" + str2.slice(2);
  }
  return expand_(escapeBraces(str2), max, maxLength, true).map(unescapeBraces);
}
function embrace(str2) {
  return "{" + str2 + "}";
}
function isPadded(el) {
  return /^-?0\d/.test(el);
}
function lte(i, y) {
  return i <= y;
}
function gte(i, y) {
  return i >= y;
}
function combine(acc, pre, values, max, maxLength, dropEmpties) {
  const out = [];
  let length = 0;
  for (let a = 0; a < acc.length; a++) {
    for (let v = 0; v < values.length; v++) {
      if (out.length >= max)
        return out;
      const expansion = acc[a] + pre + values[v];
      if (dropEmpties && !expansion)
        continue;
      if (length + expansion.length > maxLength)
        return out;
      out.push(expansion);
      length += expansion.length;
    }
  }
  return out;
}
function expandSequence(body, isAlphaSequence, max, maxLength) {
  const n = body.split(/\.\./);
  const N = [];
  if (n[0] === void 0 || n[1] === void 0) {
    return N;
  }
  const x = numeric(n[0]);
  const y = numeric(n[1]);
  const width = Math.max(n[0].length, n[1].length);
  let incr = n.length === 3 && n[2] !== void 0 ? Math.max(Math.abs(numeric(n[2])), 1) : 1;
  let test = lte;
  const reverse = y < x;
  if (reverse) {
    incr *= -1;
    test = gte;
  }
  const pad = n.some(isPadded);
  let length = 0;
  for (let i = x; test(i, y) && N.length < max; i += incr) {
    let c;
    if (isAlphaSequence) {
      c = String.fromCharCode(i);
      if (c === "\\") {
        c = "";
      }
    } else {
      c = String(i);
      if (pad) {
        const need = width - c.length;
        if (need > 0) {
          const z = new Array(need + 1).join("0");
          if (i < 0) {
            c = "-" + z + c.slice(1);
          } else {
            c = z + c;
          }
        }
      }
    }
    if (length + c.length > maxLength)
      break;
    N.push(c);
    length += c.length;
  }
  return N;
}
function expand_(str2, max, maxLength, isTop) {
  let acc = [""];
  let dropEmpties = false;
  let firstGroup = true;
  for (; ; ) {
    const m = balanced("{", "}", str2);
    if (!m) {
      return combine(acc, str2, [""], max, maxLength, dropEmpties);
    }
    const pre = m.pre;
    if (/\$$/.test(pre)) {
      acc = combine(acc, pre + "{" + m.body + "}", [""], max, maxLength, dropEmpties && !m.post.length);
      firstGroup = false;
      if (!m.post.length)
        break;
      str2 = m.post;
      continue;
    }
    const isNumericSequence = /^-?\d+\.\.-?\d+(?:\.\.-?\d+)?$/.test(m.body);
    const isAlphaSequence = /^[a-zA-Z]\.\.[a-zA-Z](?:\.\.-?\d+)?$/.test(m.body);
    const isSequence = isNumericSequence || isAlphaSequence;
    const isOptions = m.body.indexOf(",") >= 0;
    if (!isSequence && !isOptions) {
      if (m.post.match(/,(?!,).*\}/)) {
        str2 = m.pre + "{" + m.body + escClose + m.post;
        isTop = true;
        continue;
      }
      return combine(acc, pre + "{" + m.body + "}" + m.post, [""], max, maxLength, dropEmpties);
    }
    if (firstGroup) {
      dropEmpties = isTop && !isSequence;
      firstGroup = false;
    }
    let values;
    if (isSequence) {
      values = expandSequence(m.body, isAlphaSequence, max, maxLength);
    } else {
      let n = parseCommaParts(m.body);
      if (n.length === 1 && n[0] !== void 0) {
        n = expand_(n[0], max, maxLength, false).map(embrace);
        if (n.length === 1) {
          acc = combine(acc, pre + n[0], [""], max, maxLength, dropEmpties && !m.post.length);
          if (!m.post.length)
            break;
          str2 = m.post;
          continue;
        }
      }
      let dropsEmpties = dropEmpties && !m.post.length && !pre;
      for (let d = 0; dropsEmpties && d < acc.length; d++) {
        if (acc[d]) {
          dropsEmpties = false;
        }
      }
      values = [];
      let valuesLength = 0;
      outer: for (let j = 0; j < n.length; j++) {
        const expanded = expand_(n[j], max, maxLength, false);
        for (let k = 0; k < expanded.length; k++) {
          const v = expanded[k];
          if (dropsEmpties && !v)
            continue;
          if (values.length >= max || valuesLength + v.length > maxLength) {
            break outer;
          }
          values.push(v);
          valuesLength += v.length;
        }
      }
    }
    acc = combine(acc, pre, values, max, maxLength, dropEmpties && !m.post.length);
    if (!m.post.length)
      break;
    str2 = m.post;
  }
  return acc;
}

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/minimatch/dist/esm/assert-valid-pattern.js
var MAX_PATTERN_LENGTH = 1024 * 64;
var assertValidPattern = (pattern) => {
  if (typeof pattern !== "string") {
    throw new TypeError("invalid pattern");
  }
  if (pattern.length > MAX_PATTERN_LENGTH) {
    throw new TypeError("pattern is too long");
  }
};

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/minimatch/dist/esm/brace-expressions.js
var posixClasses = {
  "[:alnum:]": ["\\p{L}\\p{Nl}\\p{Nd}", true],
  "[:alpha:]": ["\\p{L}\\p{Nl}", true],
  "[:ascii:]": ["\\x00-\\x7f", false],
  "[:blank:]": ["\\p{Zs}\\t", true],
  "[:cntrl:]": ["\\p{Cc}", true],
  "[:digit:]": ["\\p{Nd}", true],
  "[:graph:]": ["\\p{Z}\\p{C}", true, true],
  "[:lower:]": ["\\p{Ll}", true],
  "[:print:]": ["\\p{C}", true],
  "[:punct:]": ["\\p{P}", true],
  "[:space:]": ["\\p{Z}\\t\\r\\n\\v\\f", true],
  "[:upper:]": ["\\p{Lu}", true],
  "[:word:]": ["\\p{L}\\p{Nl}\\p{Nd}\\p{Pc}", true],
  "[:xdigit:]": ["A-Fa-f0-9", false]
};
var braceEscape = (s) => s.replace(/[[\]\\-]/g, "\\$&");
var regexpEscape = (s) => s.replace(/[-[\]{}()*+?.,\\^$|#\s]/g, "\\$&");
var rangesToString = (ranges) => ranges.join("");
var parseClass = (glob, position) => {
  const pos = position;
  if (glob.charAt(pos) !== "[") {
    throw new Error("not in a brace expression");
  }
  const ranges = [];
  const negs = [];
  let i = pos + 1;
  let sawStart = false;
  let uflag = false;
  let escaping = false;
  let negate = false;
  let endPos = pos;
  let rangeStart = "";
  WHILE: while (i < glob.length) {
    const c = glob.charAt(i);
    if ((c === "!" || c === "^") && i === pos + 1) {
      negate = true;
      i++;
      continue;
    }
    if (c === "]" && sawStart && !escaping) {
      endPos = i + 1;
      break;
    }
    sawStart = true;
    if (c === "\\") {
      if (!escaping) {
        escaping = true;
        i++;
        continue;
      }
    }
    if (c === "[" && !escaping) {
      for (const [cls, [unip, u, neg]] of Object.entries(posixClasses)) {
        if (glob.startsWith(cls, i)) {
          if (rangeStart) {
            return ["$.", false, glob.length - pos, true];
          }
          i += cls.length;
          if (neg)
            negs.push(unip);
          else
            ranges.push(unip);
          uflag = uflag || u;
          continue WHILE;
        }
      }
    }
    escaping = false;
    if (rangeStart) {
      if (c > rangeStart) {
        ranges.push(braceEscape(rangeStart) + "-" + braceEscape(c));
      } else if (c === rangeStart) {
        ranges.push(braceEscape(c));
      }
      rangeStart = "";
      i++;
      continue;
    }
    if (glob.startsWith("-]", i + 1)) {
      ranges.push(braceEscape(c + "-"));
      i += 2;
      continue;
    }
    if (glob.startsWith("-", i + 1)) {
      rangeStart = c;
      i += 2;
      continue;
    }
    ranges.push(braceEscape(c));
    i++;
  }
  if (endPos < i) {
    return ["", false, 0, false];
  }
  if (!ranges.length && !negs.length) {
    return ["$.", false, glob.length - pos, true];
  }
  if (negs.length === 0 && ranges.length === 1 && /^\\?.$/.test(ranges[0]) && !negate) {
    const r = ranges[0].length === 2 ? ranges[0].slice(-1) : ranges[0];
    return [regexpEscape(r), false, endPos - pos, false];
  }
  const sranges = "[" + (negate ? "^" : "") + rangesToString(ranges) + "]";
  const snegs = "[" + (negate ? "" : "^") + rangesToString(negs) + "]";
  const comb = ranges.length && negs.length ? "(" + sranges + "|" + snegs + ")" : ranges.length ? sranges : snegs;
  return [comb, uflag, endPos - pos, true];
};

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/minimatch/dist/esm/unescape.js
var unescape = (s, { windowsPathsNoEscape = false, magicalBraces = true } = {}) => {
  if (magicalBraces) {
    return windowsPathsNoEscape ? s.replace(/\[([^/\\])\]/g, "$1") : s.replace(/((?!\\).|^)\[([^/\\])\]/g, "$1$2").replace(/\\([^/])/g, "$1");
  }
  return windowsPathsNoEscape ? s.replace(/\[([^/\\{}])\]/g, "$1") : s.replace(/((?!\\).|^)\[([^/\\{}])\]/g, "$1$2").replace(/\\([^/{}])/g, "$1");
};

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/minimatch/dist/esm/ast.js
var _a;
var types = /* @__PURE__ */ new Set(["!", "?", "+", "*", "@"]);
var isExtglobType = (c) => types.has(c);
var isExtglobAST = (c) => isExtglobType(c.type);
var adoptionMap = /* @__PURE__ */ new Map([
  ["!", ["@"]],
  ["?", ["?", "@"]],
  ["@", ["@"]],
  ["*", ["*", "+", "?", "@"]],
  ["+", ["+", "@"]]
]);
var adoptionWithSpaceMap = /* @__PURE__ */ new Map([
  ["!", ["?"]],
  ["@", ["?"]],
  ["+", ["?", "*"]]
]);
var adoptionAnyMap = /* @__PURE__ */ new Map([
  ["!", ["?", "@"]],
  ["?", ["?", "@"]],
  ["@", ["?", "@"]],
  ["*", ["*", "+", "?", "@"]],
  ["+", ["+", "@", "?", "*"]]
]);
var usurpMap = /* @__PURE__ */ new Map([
  ["!", /* @__PURE__ */ new Map([["!", "@"]])],
  [
    "?",
    /* @__PURE__ */ new Map([
      ["*", "*"],
      ["+", "*"]
    ])
  ],
  [
    "@",
    /* @__PURE__ */ new Map([
      ["!", "!"],
      ["?", "?"],
      ["@", "@"],
      ["*", "*"],
      ["+", "+"]
    ])
  ],
  [
    "+",
    /* @__PURE__ */ new Map([
      ["?", "*"],
      ["*", "*"]
    ])
  ]
]);
var startNoTraversal = "(?!(?:^|/)\\.\\.?(?:$|/))";
var startNoDot = "(?!\\.)";
var addPatternStart = /* @__PURE__ */ new Set(["[", "."]);
var justDots = /* @__PURE__ */ new Set(["..", "."]);
var reSpecials = new Set("().*{}+?[]^$\\!");
var regExpEscape = (s) => s.replace(/[-[\]{}()*+?.,\\^$|#\s]/g, "\\$&");
var qmark = "[^/]";
var star = qmark + "*?";
var starNoEmpty = qmark + "+?";
var ID = 0;
var AST = class {
  type;
  #root;
  #hasMagic;
  #uflag = false;
  #parts = [];
  #parent;
  #parentIndex;
  #negs;
  #filledNegs = false;
  #options;
  #toString;
  // set to true if it's an extglob with no children
  // (which really means one child of '')
  #emptyExt = false;
  id = ++ID;
  get depth() {
    return (this.#parent?.depth ?? -1) + 1;
  }
  [Symbol.for("nodejs.util.inspect.custom")]() {
    return {
      "@@type": "AST",
      id: this.id,
      type: this.type,
      root: this.#root.id,
      parent: this.#parent?.id,
      depth: this.depth,
      partsLength: this.#parts.length,
      parts: this.#parts
    };
  }
  constructor(type, parent, options = {}) {
    this.type = type;
    if (type)
      this.#hasMagic = true;
    this.#parent = parent;
    this.#root = this.#parent ? this.#parent.#root : this;
    this.#options = this.#root === this ? options : this.#root.#options;
    this.#negs = this.#root === this ? [] : this.#root.#negs;
    if (type === "!" && !this.#root.#filledNegs)
      this.#negs.push(this);
    this.#parentIndex = this.#parent ? this.#parent.#parts.length : 0;
  }
  get hasMagic() {
    if (this.#hasMagic !== void 0)
      return this.#hasMagic;
    for (const p of this.#parts) {
      if (typeof p === "string")
        continue;
      if (p.type || p.hasMagic)
        return this.#hasMagic = true;
    }
    return this.#hasMagic;
  }
  // reconstructs the pattern
  toString() {
    return this.#toString !== void 0 ? this.#toString : !this.type ? this.#toString = this.#parts.map((p) => String(p)).join("") : this.#toString = this.type + "(" + this.#parts.map((p) => String(p)).join("|") + ")";
  }
  #fillNegs() {
    if (this !== this.#root)
      throw new Error("should only call on root");
    if (this.#filledNegs)
      return this;
    this.toString();
    this.#filledNegs = true;
    let n;
    while (n = this.#negs.pop()) {
      if (n.type !== "!")
        continue;
      let p = n;
      let pp = p.#parent;
      while (pp) {
        for (let i = p.#parentIndex + 1; !pp.type && i < pp.#parts.length; i++) {
          for (const part of n.#parts) {
            if (typeof part === "string") {
              throw new Error("string part in extglob AST??");
            }
            part.copyIn(pp.#parts[i]);
          }
        }
        p = pp;
        pp = p.#parent;
      }
    }
    return this;
  }
  push(...parts) {
    for (const p of parts) {
      if (p === "")
        continue;
      if (typeof p !== "string" && !(p instanceof _a && p.#parent === this)) {
        throw new Error("invalid part: " + p);
      }
      this.#parts.push(p);
    }
  }
  toJSON() {
    const ret = this.type === null ? this.#parts.slice().map((p) => typeof p === "string" ? p : p.toJSON()) : [this.type, ...this.#parts.map((p) => p.toJSON())];
    if (this.isStart() && !this.type)
      ret.unshift([]);
    if (this.isEnd() && (this === this.#root || this.#root.#filledNegs && this.#parent?.type === "!")) {
      ret.push({});
    }
    return ret;
  }
  isStart() {
    if (this.#root === this)
      return true;
    if (!this.#parent?.isStart())
      return false;
    if (this.#parentIndex === 0)
      return true;
    const p = this.#parent;
    for (let i = 0; i < this.#parentIndex; i++) {
      const pp = p.#parts[i];
      if (!(pp instanceof _a && pp.type === "!")) {
        return false;
      }
    }
    return true;
  }
  isEnd() {
    if (this.#root === this)
      return true;
    if (this.#parent?.type === "!")
      return true;
    if (!this.#parent?.isEnd())
      return false;
    if (!this.type)
      return this.#parent?.isEnd();
    const pl = this.#parent ? this.#parent.#parts.length : 0;
    return this.#parentIndex === pl - 1;
  }
  copyIn(part) {
    if (typeof part === "string")
      this.push(part);
    else
      this.push(part.clone(this));
  }
  clone(parent) {
    const c = new _a(this.type, parent);
    for (const p of this.#parts) {
      c.copyIn(p);
    }
    return c;
  }
  static #parseAST(str2, ast, pos, opt, extDepth) {
    const maxDepth = opt.maxExtglobRecursion ?? 2;
    let escaping = false;
    let inBrace = false;
    let braceStart = -1;
    let braceNeg = false;
    if (ast.type === null) {
      let i2 = pos;
      let acc2 = "";
      while (i2 < str2.length) {
        const c = str2.charAt(i2++);
        if (escaping || c === "\\") {
          escaping = !escaping;
          acc2 += c;
          continue;
        }
        if (inBrace) {
          if (i2 === braceStart + 1) {
            if (c === "^" || c === "!") {
              braceNeg = true;
            }
          } else if (c === "]" && !(i2 === braceStart + 2 && braceNeg)) {
            inBrace = false;
          }
          acc2 += c;
          continue;
        } else if (c === "[") {
          inBrace = true;
          braceStart = i2;
          braceNeg = false;
          acc2 += c;
          continue;
        }
        const doRecurse = !opt.noext && isExtglobType(c) && str2.charAt(i2) === "(" && extDepth <= maxDepth;
        if (doRecurse) {
          ast.push(acc2);
          acc2 = "";
          const ext2 = new _a(c, ast);
          i2 = _a.#parseAST(str2, ext2, i2, opt, extDepth + 1);
          ast.push(ext2);
          continue;
        }
        acc2 += c;
      }
      ast.push(acc2);
      return i2;
    }
    let i = pos + 1;
    let part = new _a(null, ast);
    const parts = [];
    let acc = "";
    while (i < str2.length) {
      const c = str2.charAt(i++);
      if (escaping || c === "\\") {
        escaping = !escaping;
        acc += c;
        continue;
      }
      if (inBrace) {
        if (i === braceStart + 1) {
          if (c === "^" || c === "!") {
            braceNeg = true;
          }
        } else if (c === "]" && !(i === braceStart + 2 && braceNeg)) {
          inBrace = false;
        }
        acc += c;
        continue;
      } else if (c === "[") {
        inBrace = true;
        braceStart = i;
        braceNeg = false;
        acc += c;
        continue;
      }
      const doRecurse = !opt.noext && isExtglobType(c) && str2.charAt(i) === "(" && /* c8 ignore start - the maxDepth is sufficient here */
      (extDepth <= maxDepth || ast && ast.#canAdoptType(c));
      if (doRecurse) {
        const depthAdd = ast && ast.#canAdoptType(c) ? 0 : 1;
        part.push(acc);
        acc = "";
        const ext2 = new _a(c, part);
        part.push(ext2);
        i = _a.#parseAST(str2, ext2, i, opt, extDepth + depthAdd);
        continue;
      }
      if (c === "|") {
        part.push(acc);
        acc = "";
        parts.push(part);
        part = new _a(null, ast);
        continue;
      }
      if (c === ")") {
        if (acc === "" && ast.#parts.length === 0) {
          ast.#emptyExt = true;
        }
        part.push(acc);
        acc = "";
        ast.push(...parts, part);
        return i;
      }
      acc += c;
    }
    ast.type = null;
    ast.#hasMagic = void 0;
    ast.#parts = [str2.substring(pos - 1)];
    return i;
  }
  #canAdoptWithSpace(child) {
    return this.#canAdopt(child, adoptionWithSpaceMap);
  }
  #canAdopt(child, map = adoptionMap) {
    if (!child || typeof child !== "object" || child.type !== null || child.#parts.length !== 1 || this.type === null) {
      return false;
    }
    const gc = child.#parts[0];
    if (!gc || typeof gc !== "object" || gc.type === null) {
      return false;
    }
    return this.#canAdoptType(gc.type, map);
  }
  #canAdoptType(c, map = adoptionAnyMap) {
    return !!map.get(this.type)?.includes(c);
  }
  #adoptWithSpace(child, index) {
    const gc = child.#parts[0];
    const blank = new _a(null, gc, this.options);
    blank.#parts.push("");
    gc.push(blank);
    this.#adopt(child, index);
  }
  #adopt(child, index) {
    const gc = child.#parts[0];
    this.#parts.splice(index, 1, ...gc.#parts);
    for (const p of gc.#parts) {
      if (typeof p === "object")
        p.#parent = this;
    }
    this.#toString = void 0;
  }
  #canUsurpType(c) {
    const m = usurpMap.get(this.type);
    return !!m?.has(c);
  }
  #canUsurp(child) {
    if (!child || typeof child !== "object" || child.type !== null || child.#parts.length !== 1 || this.type === null || this.#parts.length !== 1) {
      return false;
    }
    const gc = child.#parts[0];
    if (!gc || typeof gc !== "object" || gc.type === null) {
      return false;
    }
    return this.#canUsurpType(gc.type);
  }
  #usurp(child) {
    const m = usurpMap.get(this.type);
    const gc = child.#parts[0];
    const nt = m?.get(gc.type);
    if (!nt)
      return false;
    this.#parts = gc.#parts;
    for (const p of this.#parts) {
      if (typeof p === "object") {
        p.#parent = this;
      }
    }
    this.type = nt;
    this.#toString = void 0;
    this.#emptyExt = false;
  }
  static fromGlob(pattern, options = {}) {
    const ast = new _a(null, void 0, options);
    _a.#parseAST(pattern, ast, 0, options, 0);
    return ast;
  }
  // returns the regular expression if there's magic, or the unescaped
  // string if not.
  toMMPattern() {
    if (this !== this.#root)
      return this.#root.toMMPattern();
    const glob = this.toString();
    const [re, body, hasMagic, uflag] = this.toRegExpSource();
    const anyMagic = hasMagic || this.#hasMagic || this.#options.nocase && !this.#options.nocaseMagicOnly && glob.toUpperCase() !== glob.toLowerCase();
    if (!anyMagic) {
      return body;
    }
    const flags = (this.#options.nocase ? "i" : "") + (uflag ? "u" : "");
    return Object.assign(new RegExp(`^${re}$`, flags), {
      _src: re,
      _glob: glob
    });
  }
  get options() {
    return this.#options;
  }
  // returns the string match, the regexp source, whether there's magic
  // in the regexp (so a regular expression is required) and whether or
  // not the uflag is needed for the regular expression (for posix classes)
  // TODO: instead of injecting the start/end at this point, just return
  // the BODY of the regexp, along with the start/end portions suitable
  // for binding the start/end in either a joined full-path makeRe context
  // (where we bind to (^|/), or a standalone matchPart context (where
  // we bind to ^, and not /).  Otherwise slashes get duped!
  //
  // In part-matching mode, the start is:
  // - if not isStart: nothing
  // - if traversal possible, but not allowed: ^(?!\.\.?$)
  // - if dots allowed or not possible: ^
  // - if dots possible and not allowed: ^(?!\.)
  // end is:
  // - if not isEnd(): nothing
  // - else: $
  //
  // In full-path matching mode, we put the slash at the START of the
  // pattern, so start is:
  // - if first pattern: same as part-matching mode
  // - if not isStart(): nothing
  // - if traversal possible, but not allowed: /(?!\.\.?(?:$|/))
  // - if dots allowed or not possible: /
  // - if dots possible and not allowed: /(?!\.)
  // end is:
  // - if last pattern, same as part-matching mode
  // - else nothing
  //
  // Always put the (?:$|/) on negated tails, though, because that has to be
  // there to bind the end of the negated pattern portion, and it's easier to
  // just stick it in now rather than try to inject it later in the middle of
  // the pattern.
  //
  // We can just always return the same end, and leave it up to the caller
  // to know whether it's going to be used joined or in parts.
  // And, if the start is adjusted slightly, can do the same there:
  // - if not isStart: nothing
  // - if traversal possible, but not allowed: (?:/|^)(?!\.\.?$)
  // - if dots allowed or not possible: (?:/|^)
  // - if dots possible and not allowed: (?:/|^)(?!\.)
  //
  // But it's better to have a simpler binding without a conditional, for
  // performance, so probably better to return both start options.
  //
  // Then the caller just ignores the end if it's not the first pattern,
  // and the start always gets applied.
  //
  // But that's always going to be $ if it's the ending pattern, or nothing,
  // so the caller can just attach $ at the end of the pattern when building.
  //
  // So the todo is:
  // - better detect what kind of start is needed
  // - return both flavors of starting pattern
  // - attach $ at the end of the pattern when creating the actual RegExp
  //
  // Ah, but wait, no, that all only applies to the root when the first pattern
  // is not an extglob. If the first pattern IS an extglob, then we need all
  // that dot prevention biz to live in the extglob portions, because eg
  // +(*|.x*) can match .xy but not .yx.
  //
  // So, return the two flavors if it's #root and the first child is not an
  // AST, otherwise leave it to the child AST to handle it, and there,
  // use the (?:^|/) style of start binding.
  //
  // Even simplified further:
  // - Since the start for a join is eg /(?!\.) and the start for a part
  // is ^(?!\.), we can just prepend (?!\.) to the pattern (either root
  // or start or whatever) and prepend ^ or / at the Regexp construction.
  toRegExpSource(allowDot) {
    const dot = allowDot ?? !!this.#options.dot;
    if (this.#root === this) {
      this.#flatten();
      this.#fillNegs();
    }
    if (!isExtglobAST(this)) {
      const noEmpty = this.isStart() && this.isEnd() && !this.#parts.some((s) => typeof s !== "string");
      const src = this.#parts.map((p) => {
        const [re, _, hasMagic, uflag] = typeof p === "string" ? _a.#parseGlob(p, this.#hasMagic, noEmpty) : p.toRegExpSource(allowDot);
        this.#hasMagic = this.#hasMagic || hasMagic;
        this.#uflag = this.#uflag || uflag;
        return re;
      }).join("");
      let start2 = "";
      if (this.isStart()) {
        if (typeof this.#parts[0] === "string") {
          const dotTravAllowed = this.#parts.length === 1 && justDots.has(this.#parts[0]);
          if (!dotTravAllowed) {
            const aps = addPatternStart;
            const needNoTrav = (
              // dots are allowed, and the pattern starts with [ or .
              dot && aps.has(src.charAt(0)) || // the pattern starts with \., and then [ or .
              src.startsWith("\\.") && aps.has(src.charAt(2)) || // the pattern starts with \.\., and then [ or .
              src.startsWith("\\.\\.") && aps.has(src.charAt(4))
            );
            const needNoDot = !dot && !allowDot && aps.has(src.charAt(0));
            start2 = needNoTrav ? startNoTraversal : needNoDot ? startNoDot : "";
          }
        }
      }
      let end = "";
      if (this.isEnd() && this.#root.#filledNegs && this.#parent?.type === "!") {
        end = "(?:$|\\/)";
      }
      const final2 = start2 + src + end;
      return [
        final2,
        unescape(src),
        this.#hasMagic = !!this.#hasMagic,
        this.#uflag
      ];
    }
    const repeated = this.type === "*" || this.type === "+";
    const start = this.type === "!" ? "(?:(?!(?:" : "(?:";
    let body = this.#partsToRegExp(dot);
    if (this.isStart() && this.isEnd() && !body && this.type !== "!") {
      const s = this.toString();
      const me = this;
      me.#parts = [s];
      me.type = null;
      me.#hasMagic = void 0;
      return [s, unescape(this.toString()), false, false];
    }
    let bodyDotAllowed = !repeated || allowDot || dot || !startNoDot ? "" : this.#partsToRegExp(true);
    if (bodyDotAllowed === body) {
      bodyDotAllowed = "";
    }
    if (bodyDotAllowed) {
      body = `(?:${body})(?:${bodyDotAllowed})*?`;
    }
    let final = "";
    if (this.type === "!" && this.#emptyExt) {
      final = (this.isStart() && !dot ? startNoDot : "") + starNoEmpty;
    } else {
      const close = this.type === "!" ? (
        // !() must match something,but !(x) can match ''
        "))" + (this.isStart() && !dot && !allowDot ? startNoDot : "") + star + ")"
      ) : this.type === "@" ? ")" : this.type === "?" ? ")?" : this.type === "+" && bodyDotAllowed ? ")" : this.type === "*" && bodyDotAllowed ? `)?` : `)${this.type}`;
      final = start + body + close;
    }
    return [
      final,
      unescape(body),
      this.#hasMagic = !!this.#hasMagic,
      this.#uflag
    ];
  }
  #flatten() {
    if (!isExtglobAST(this)) {
      for (const p of this.#parts) {
        if (typeof p === "object") {
          p.#flatten();
        }
      }
    } else {
      let iterations = 0;
      let done = false;
      do {
        done = true;
        for (let i = 0; i < this.#parts.length; i++) {
          const c = this.#parts[i];
          if (typeof c === "object") {
            c.#flatten();
            if (this.#canAdopt(c)) {
              done = false;
              this.#adopt(c, i);
            } else if (this.#canAdoptWithSpace(c)) {
              done = false;
              this.#adoptWithSpace(c, i);
            } else if (this.#canUsurp(c)) {
              done = false;
              this.#usurp(c);
            }
          }
        }
      } while (!done && ++iterations < 10);
    }
    this.#toString = void 0;
  }
  #partsToRegExp(dot) {
    return this.#parts.map((p) => {
      if (typeof p === "string") {
        throw new Error("string type in extglob ast??");
      }
      const [re, _, _hasMagic, uflag] = p.toRegExpSource(dot);
      this.#uflag = this.#uflag || uflag;
      return re;
    }).filter((p) => !(this.isStart() && this.isEnd()) || !!p).join("|");
  }
  static #parseGlob(glob, hasMagic, noEmpty = false) {
    let escaping = false;
    let re = "";
    let uflag = false;
    let inStar = false;
    for (let i = 0; i < glob.length; i++) {
      const c = glob.charAt(i);
      if (escaping) {
        escaping = false;
        re += (reSpecials.has(c) ? "\\" : "") + c;
        continue;
      }
      if (c === "*") {
        if (inStar)
          continue;
        inStar = true;
        re += noEmpty && /^[*]+$/.test(glob) ? starNoEmpty : star;
        hasMagic = true;
        continue;
      } else {
        inStar = false;
      }
      if (c === "\\") {
        if (i === glob.length - 1) {
          re += "\\\\";
        } else {
          escaping = true;
        }
        continue;
      }
      if (c === "[") {
        const [src, needUflag, consumed, magic] = parseClass(glob, i);
        if (consumed) {
          re += src;
          uflag = uflag || needUflag;
          i += consumed - 1;
          hasMagic = hasMagic || magic;
          continue;
        }
      }
      if (c === "?") {
        re += qmark;
        hasMagic = true;
        continue;
      }
      re += regExpEscape(c);
    }
    return [re, unescape(glob), !!hasMagic, uflag];
  }
};
_a = AST;

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/minimatch/dist/esm/escape.js
var escape = (s, { windowsPathsNoEscape = false, magicalBraces = false } = {}) => {
  if (magicalBraces) {
    return windowsPathsNoEscape ? s.replace(/[?*()[\]{}]/g, "[$&]") : s.replace(/[?*()[\]\\{}]/g, "\\$&");
  }
  return windowsPathsNoEscape ? s.replace(/[?*()[\]]/g, "[$&]") : s.replace(/[?*()[\]\\]/g, "\\$&");
};

// ../../../tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules/minimatch/dist/esm/index.js
var minimatch = (p, pattern, options = {}) => {
  assertValidPattern(pattern);
  if (!options.nocomment && pattern.charAt(0) === "#") {
    return false;
  }
  return new Minimatch(pattern, options).match(p);
};
var starDotExtRE = /^\*+([^+@!?*[(]*)$/;
var starDotExtTest = (ext2) => (f) => !f.startsWith(".") && f.endsWith(ext2);
var starDotExtTestDot = (ext2) => (f) => f.endsWith(ext2);
var starDotExtTestNocase = (ext2) => {
  ext2 = ext2.toLowerCase();
  return (f) => !f.startsWith(".") && f.toLowerCase().endsWith(ext2);
};
var starDotExtTestNocaseDot = (ext2) => {
  ext2 = ext2.toLowerCase();
  return (f) => f.toLowerCase().endsWith(ext2);
};
var starDotStarRE = /^\*+\.\*+$/;
var starDotStarTest = (f) => !f.startsWith(".") && f.includes(".");
var starDotStarTestDot = (f) => f !== "." && f !== ".." && f.includes(".");
var dotStarRE = /^\.\*+$/;
var dotStarTest = (f) => f !== "." && f !== ".." && f.startsWith(".");
var starRE = /^\*+$/;
var starTest = (f) => f.length !== 0 && !f.startsWith(".");
var starTestDot = (f) => f.length !== 0 && f !== "." && f !== "..";
var qmarksRE = /^\?+([^+@!?*[(]*)?$/;
var qmarksTestNocase = ([$0, ext2 = ""]) => {
  const noext = qmarksTestNoExt([$0]);
  if (!ext2)
    return noext;
  ext2 = ext2.toLowerCase();
  return (f) => noext(f) && f.toLowerCase().endsWith(ext2);
};
var qmarksTestNocaseDot = ([$0, ext2 = ""]) => {
  const noext = qmarksTestNoExtDot([$0]);
  if (!ext2)
    return noext;
  ext2 = ext2.toLowerCase();
  return (f) => noext(f) && f.toLowerCase().endsWith(ext2);
};
var qmarksTestDot = ([$0, ext2 = ""]) => {
  const noext = qmarksTestNoExtDot([$0]);
  return !ext2 ? noext : (f) => noext(f) && f.endsWith(ext2);
};
var qmarksTest = ([$0, ext2 = ""]) => {
  const noext = qmarksTestNoExt([$0]);
  return !ext2 ? noext : (f) => noext(f) && f.endsWith(ext2);
};
var qmarksTestNoExt = ([$0]) => {
  const len = $0.length;
  return (f) => f.length === len && !f.startsWith(".");
};
var qmarksTestNoExtDot = ([$0]) => {
  const len = $0.length;
  return (f) => f.length === len && f !== "." && f !== "..";
};
var defaultPlatform = typeof process === "object" && process ? typeof process.env === "object" && process.env && process.env.__MINIMATCH_TESTING_PLATFORM__ || process.platform : "posix";
var path = {
  win32: { sep: "\\" },
  posix: { sep: "/" }
};
var sep3 = defaultPlatform === "win32" ? path.win32.sep : path.posix.sep;
minimatch.sep = sep3;
var GLOBSTAR = Symbol("globstar **");
minimatch.GLOBSTAR = GLOBSTAR;
var qmark2 = "[^/]";
var star2 = qmark2 + "*?";
var twoStarDot = "(?:(?!(?:\\/|^)(?:\\.{1,2})($|\\/)).)*?";
var twoStarNoDot = "(?:(?!(?:\\/|^)\\.).)*?";
var filter = (pattern, options = {}) => (p) => minimatch(p, pattern, options);
minimatch.filter = filter;
var ext = (a, b = {}) => Object.assign({}, a, b);
var defaults = (def) => {
  if (!def || typeof def !== "object" || !Object.keys(def).length) {
    return minimatch;
  }
  const orig = minimatch;
  const m = (p, pattern, options = {}) => orig(p, pattern, ext(def, options));
  return Object.assign(m, {
    Minimatch: class Minimatch extends orig.Minimatch {
      constructor(pattern, options = {}) {
        super(pattern, ext(def, options));
      }
      static defaults(options) {
        return orig.defaults(ext(def, options)).Minimatch;
      }
    },
    AST: class AST extends orig.AST {
      /* c8 ignore start */
      constructor(type, parent, options = {}) {
        super(type, parent, ext(def, options));
      }
      /* c8 ignore stop */
      static fromGlob(pattern, options = {}) {
        return orig.AST.fromGlob(pattern, ext(def, options));
      }
    },
    unescape: (s, options = {}) => orig.unescape(s, ext(def, options)),
    escape: (s, options = {}) => orig.escape(s, ext(def, options)),
    filter: (pattern, options = {}) => orig.filter(pattern, ext(def, options)),
    defaults: (options) => orig.defaults(ext(def, options)),
    makeRe: (pattern, options = {}) => orig.makeRe(pattern, ext(def, options)),
    braceExpand: (pattern, options = {}) => orig.braceExpand(pattern, ext(def, options)),
    match: (list, pattern, options = {}) => orig.match(list, pattern, ext(def, options)),
    sep: orig.sep,
    GLOBSTAR
  });
};
minimatch.defaults = defaults;
var braceExpand = (pattern, options = {}) => {
  assertValidPattern(pattern);
  if (options.nobrace || !/\{(?:(?!\{).)*\}/.test(pattern)) {
    return [pattern];
  }
  return expand(pattern, { max: options.braceExpandMax });
};
minimatch.braceExpand = braceExpand;
var makeRe = (pattern, options = {}) => new Minimatch(pattern, options).makeRe();
minimatch.makeRe = makeRe;
var match = (list, pattern, options = {}) => {
  const mm = new Minimatch(pattern, options);
  list = list.filter((f) => mm.match(f));
  if (mm.options.nonull && !list.length) {
    list.push(pattern);
  }
  return list;
};
minimatch.match = match;
var globMagic = /[?*]|[+@!]\(.*?\)|\[|\]/;
var regExpEscape2 = (s) => s.replace(/[-[\]{}()*+?.,\\^$|#\s]/g, "\\$&");
var Minimatch = class {
  options;
  set;
  pattern;
  windowsPathsNoEscape;
  nonegate;
  negate;
  comment;
  empty;
  preserveMultipleSlashes;
  partial;
  globSet;
  globParts;
  nocase;
  isWindows;
  platform;
  windowsNoMagicRoot;
  maxGlobstarRecursion;
  regexp;
  constructor(pattern, options = {}) {
    assertValidPattern(pattern);
    options = options || {};
    this.options = options;
    this.maxGlobstarRecursion = options.maxGlobstarRecursion ?? 200;
    this.pattern = pattern;
    this.platform = options.platform || defaultPlatform;
    this.isWindows = this.platform === "win32";
    const awe = "allowWindowsEscape";
    this.windowsPathsNoEscape = !!options.windowsPathsNoEscape || options[awe] === false;
    if (this.windowsPathsNoEscape) {
      this.pattern = this.pattern.replace(/\\/g, "/");
    }
    this.preserveMultipleSlashes = !!options.preserveMultipleSlashes;
    this.regexp = null;
    this.negate = false;
    this.nonegate = !!options.nonegate;
    this.comment = false;
    this.empty = false;
    this.partial = !!options.partial;
    this.nocase = !!this.options.nocase;
    this.windowsNoMagicRoot = options.windowsNoMagicRoot !== void 0 ? options.windowsNoMagicRoot : !!(this.isWindows && this.nocase);
    this.globSet = [];
    this.globParts = [];
    this.set = [];
    this.make();
  }
  hasMagic() {
    if (this.options.magicalBraces && this.set.length > 1) {
      return true;
    }
    for (const pattern of this.set) {
      for (const part of pattern) {
        if (typeof part !== "string")
          return true;
      }
    }
    return false;
  }
  debug(..._) {
  }
  make() {
    const pattern = this.pattern;
    const options = this.options;
    if (!options.nocomment && pattern.charAt(0) === "#") {
      this.comment = true;
      return;
    }
    if (!pattern) {
      this.empty = true;
      return;
    }
    this.parseNegate();
    this.globSet = [...new Set(this.braceExpand())];
    if (options.debug) {
      this.debug = (...args) => console.error(...args);
    }
    this.debug(this.pattern, this.globSet);
    const rawGlobParts = this.globSet.map((s) => this.slashSplit(s));
    this.globParts = this.preprocess(rawGlobParts);
    this.debug(this.pattern, this.globParts);
    let set = this.globParts.map((s, _, __) => {
      if (this.isWindows && this.windowsNoMagicRoot) {
        const isUNC = s[0] === "" && s[1] === "" && (s[2] === "?" || !globMagic.test(s[2])) && !globMagic.test(s[3]);
        const isDrive = /^[a-z]:/i.test(s[0]);
        if (isUNC) {
          return [
            ...s.slice(0, 4),
            ...s.slice(4).map((ss) => this.parse(ss))
          ];
        } else if (isDrive) {
          return [s[0], ...s.slice(1).map((ss) => this.parse(ss))];
        }
      }
      return s.map((ss) => this.parse(ss));
    });
    this.debug(this.pattern, set);
    this.set = set.filter((s) => s.indexOf(false) === -1);
    if (this.isWindows) {
      for (let i = 0; i < this.set.length; i++) {
        const p = this.set[i];
        if (p[0] === "" && p[1] === "" && this.globParts[i][2] === "?" && typeof p[3] === "string" && /^[a-z]:$/i.test(p[3])) {
          p[2] = "?";
        }
      }
    }
    this.debug(this.pattern, this.set);
  }
  // various transforms to equivalent pattern sets that are
  // faster to process in a filesystem walk.  The goal is to
  // eliminate what we can, and push all ** patterns as far
  // to the right as possible, even if it increases the number
  // of patterns that we have to process.
  preprocess(globParts) {
    if (this.options.noglobstar) {
      for (const partset of globParts) {
        for (let j = 0; j < partset.length; j++) {
          if (partset[j] === "**") {
            partset[j] = "*";
          }
        }
      }
    }
    const { optimizationLevel = 1 } = this.options;
    if (optimizationLevel >= 2) {
      globParts = this.firstPhasePreProcess(globParts);
      globParts = this.secondPhasePreProcess(globParts);
    } else if (optimizationLevel >= 1) {
      globParts = this.levelOneOptimize(globParts);
    } else {
      globParts = this.adjascentGlobstarOptimize(globParts);
    }
    return globParts;
  }
  // just get rid of adjascent ** portions
  adjascentGlobstarOptimize(globParts) {
    return globParts.map((parts) => {
      let gs = -1;
      while (-1 !== (gs = parts.indexOf("**", gs + 1))) {
        let i = gs;
        while (parts[i + 1] === "**") {
          i++;
        }
        if (i !== gs) {
          parts.splice(gs, i - gs);
        }
      }
      return parts;
    });
  }
  // get rid of adjascent ** and resolve .. portions
  levelOneOptimize(globParts) {
    return globParts.map((parts) => {
      parts = parts.reduce((set, part) => {
        const prev = set[set.length - 1];
        if (part === "**" && prev === "**") {
          return set;
        }
        if (part === "..") {
          if (prev && prev !== ".." && prev !== "." && prev !== "**") {
            set.pop();
            return set;
          }
        }
        set.push(part);
        return set;
      }, []);
      return parts.length === 0 ? [""] : parts;
    });
  }
  levelTwoFileOptimize(parts) {
    if (!Array.isArray(parts)) {
      parts = this.slashSplit(parts);
    }
    let didSomething = false;
    do {
      didSomething = false;
      if (!this.preserveMultipleSlashes) {
        for (let i = 1; i < parts.length - 1; i++) {
          const p = parts[i];
          if (i === 1 && p === "" && parts[0] === "")
            continue;
          if (p === "." || p === "") {
            didSomething = true;
            parts.splice(i, 1);
            i--;
          }
        }
        if (parts[0] === "." && parts.length === 2 && (parts[1] === "." || parts[1] === "")) {
          didSomething = true;
          parts.pop();
        }
      }
      let dd = 0;
      while (-1 !== (dd = parts.indexOf("..", dd + 1))) {
        const p = parts[dd - 1];
        if (p && p !== "." && p !== ".." && p !== "**" && !(this.isWindows && /^[a-z]:$/i.test(p))) {
          didSomething = true;
          parts.splice(dd - 1, 2);
          dd -= 2;
        }
      }
    } while (didSomething);
    return parts.length === 0 ? [""] : parts;
  }
  // First phase: single-pattern processing
  // <pre> is 1 or more portions
  // <rest> is 1 or more portions
  // <p> is any portion other than ., .., '', or **
  // <e> is . or ''
  //
  // **/.. is *brutal* for filesystem walking performance, because
  // it effectively resets the recursive walk each time it occurs,
  // and ** cannot be reduced out by a .. pattern part like a regexp
  // or most strings (other than .., ., and '') can be.
  //
  // <pre>/**/../<p>/<p>/<rest> -> {<pre>/../<p>/<p>/<rest>,<pre>/**/<p>/<p>/<rest>}
  // <pre>/<e>/<rest> -> <pre>/<rest>
  // <pre>/<p>/../<rest> -> <pre>/<rest>
  // **/**/<rest> -> **/<rest>
  //
  // **/*/<rest> -> */**/<rest> <== not valid because ** doesn't follow
  // this WOULD be allowed if ** did follow symlinks, or * didn't
  firstPhasePreProcess(globParts) {
    let didSomething = false;
    do {
      didSomething = false;
      for (let parts of globParts) {
        let gs = -1;
        while (-1 !== (gs = parts.indexOf("**", gs + 1))) {
          let gss = gs;
          while (parts[gss + 1] === "**") {
            gss++;
          }
          if (gss > gs) {
            parts.splice(gs + 1, gss - gs);
          }
          let next = parts[gs + 1];
          const p = parts[gs + 2];
          const p2 = parts[gs + 3];
          if (next !== "..")
            continue;
          if (!p || p === "." || p === ".." || !p2 || p2 === "." || p2 === "..") {
            continue;
          }
          didSomething = true;
          parts.splice(gs, 1);
          const other = parts.slice(0);
          other[gs] = "**";
          globParts.push(other);
          gs--;
        }
        if (!this.preserveMultipleSlashes) {
          for (let i = 1; i < parts.length - 1; i++) {
            const p = parts[i];
            if (i === 1 && p === "" && parts[0] === "")
              continue;
            if (p === "." || p === "") {
              didSomething = true;
              parts.splice(i, 1);
              i--;
            }
          }
          if (parts[0] === "." && parts.length === 2 && (parts[1] === "." || parts[1] === "")) {
            didSomething = true;
            parts.pop();
          }
        }
        let dd = 0;
        while (-1 !== (dd = parts.indexOf("..", dd + 1))) {
          const p = parts[dd - 1];
          if (p && p !== "." && p !== ".." && p !== "**") {
            didSomething = true;
            const needDot = dd === 1 && parts[dd + 1] === "**";
            const splin = needDot ? ["."] : [];
            parts.splice(dd - 1, 2, ...splin);
            if (parts.length === 0)
              parts.push("");
            dd -= 2;
          }
        }
      }
    } while (didSomething);
    return globParts;
  }
  // second phase: multi-pattern dedupes
  // {<pre>/*/<rest>,<pre>/<p>/<rest>} -> <pre>/*/<rest>
  // {<pre>/<rest>,<pre>/<rest>} -> <pre>/<rest>
  // {<pre>/**/<rest>,<pre>/<rest>} -> <pre>/**/<rest>
  //
  // {<pre>/**/<rest>,<pre>/**/<p>/<rest>} -> <pre>/**/<rest>
  // ^-- not valid because ** doens't follow symlinks
  secondPhasePreProcess(globParts) {
    for (let i = 0; i < globParts.length - 1; i++) {
      for (let j = i + 1; j < globParts.length; j++) {
        const matched = this.partsMatch(globParts[i], globParts[j], !this.preserveMultipleSlashes);
        if (matched) {
          globParts[i] = [];
          globParts[j] = matched;
          break;
        }
      }
    }
    return globParts.filter((gs) => gs.length);
  }
  partsMatch(a, b, emptyGSMatch = false) {
    let ai = 0;
    let bi = 0;
    let result = [];
    let which = "";
    while (ai < a.length && bi < b.length) {
      if (a[ai] === b[bi]) {
        result.push(which === "b" ? b[bi] : a[ai]);
        ai++;
        bi++;
      } else if (emptyGSMatch && a[ai] === "**" && b[bi] === a[ai + 1]) {
        result.push(a[ai]);
        ai++;
      } else if (emptyGSMatch && b[bi] === "**" && a[ai] === b[bi + 1]) {
        result.push(b[bi]);
        bi++;
      } else if (a[ai] === "*" && b[bi] && (this.options.dot || !b[bi].startsWith(".")) && b[bi] !== "**") {
        if (which === "b")
          return false;
        which = "a";
        result.push(a[ai]);
        ai++;
        bi++;
      } else if (b[bi] === "*" && a[ai] && (this.options.dot || !a[ai].startsWith(".")) && a[ai] !== "**") {
        if (which === "a")
          return false;
        which = "b";
        result.push(b[bi]);
        ai++;
        bi++;
      } else {
        return false;
      }
    }
    return a.length === b.length && result;
  }
  parseNegate() {
    if (this.nonegate)
      return;
    const pattern = this.pattern;
    let negate = false;
    let negateOffset = 0;
    for (let i = 0; i < pattern.length && pattern.charAt(i) === "!"; i++) {
      negate = !negate;
      negateOffset++;
    }
    if (negateOffset)
      this.pattern = pattern.slice(negateOffset);
    this.negate = negate;
  }
  // set partial to true to test if, for example,
  // "/a/b" matches the start of "/*/b/*/d"
  // Partial means, if you run out of file before you run
  // out of pattern, then that's fine, as long as all
  // the parts match.
  matchOne(file, pattern, partial = false) {
    let fileStartIndex = 0;
    let patternStartIndex = 0;
    if (this.isWindows) {
      const fileDrive = typeof file[0] === "string" && /^[a-z]:$/i.test(file[0]);
      const fileUNC = !fileDrive && file[0] === "" && file[1] === "" && file[2] === "?" && /^[a-z]:$/i.test(file[3]);
      const patternDrive = typeof pattern[0] === "string" && /^[a-z]:$/i.test(pattern[0]);
      const patternUNC = !patternDrive && pattern[0] === "" && pattern[1] === "" && pattern[2] === "?" && typeof pattern[3] === "string" && /^[a-z]:$/i.test(pattern[3]);
      const fdi = fileUNC ? 3 : fileDrive ? 0 : void 0;
      const pdi = patternUNC ? 3 : patternDrive ? 0 : void 0;
      if (typeof fdi === "number" && typeof pdi === "number") {
        const [fd, pd] = [
          file[fdi],
          pattern[pdi]
        ];
        if (fd.toLowerCase() === pd.toLowerCase()) {
          pattern[pdi] = fd;
          patternStartIndex = pdi;
          fileStartIndex = fdi;
        }
      }
    }
    const { optimizationLevel = 1 } = this.options;
    if (optimizationLevel >= 2) {
      file = this.levelTwoFileOptimize(file);
    }
    if (pattern.includes(GLOBSTAR)) {
      return this.#matchGlobstar(file, pattern, partial, fileStartIndex, patternStartIndex);
    }
    return this.#matchOne(file, pattern, partial, fileStartIndex, patternStartIndex);
  }
  #matchGlobstar(file, pattern, partial, fileIndex, patternIndex) {
    const firstgs = pattern.indexOf(GLOBSTAR, patternIndex);
    const lastgs = pattern.lastIndexOf(GLOBSTAR);
    const [head, body, tail] = partial ? [
      pattern.slice(patternIndex, firstgs),
      pattern.slice(firstgs + 1),
      []
    ] : [
      pattern.slice(patternIndex, firstgs),
      pattern.slice(firstgs + 1, lastgs),
      pattern.slice(lastgs + 1)
    ];
    if (head.length) {
      const fileHead = file.slice(fileIndex, fileIndex + head.length);
      if (!this.#matchOne(fileHead, head, partial, 0, 0)) {
        return false;
      }
      fileIndex += head.length;
      patternIndex += head.length;
    }
    let fileTailMatch = 0;
    if (tail.length) {
      if (tail.length + fileIndex > file.length)
        return false;
      let tailStart = file.length - tail.length;
      if (this.#matchOne(file, tail, partial, tailStart, 0)) {
        fileTailMatch = tail.length;
      } else {
        if (file[file.length - 1] !== "" || fileIndex + tail.length === file.length) {
          return false;
        }
        tailStart--;
        if (!this.#matchOne(file, tail, partial, tailStart, 0)) {
          return false;
        }
        fileTailMatch = tail.length + 1;
      }
    }
    if (!body.length) {
      let sawSome = !!fileTailMatch;
      for (let i2 = fileIndex; i2 < file.length - fileTailMatch; i2++) {
        const f = String(file[i2]);
        sawSome = true;
        if (f === "." || f === ".." || !this.options.dot && f.startsWith(".")) {
          return false;
        }
      }
      return partial || sawSome;
    }
    const bodySegments = [[[], 0]];
    let currentBody = bodySegments[0];
    let nonGsParts = 0;
    const nonGsPartsSums = [0];
    for (const b of body) {
      if (b === GLOBSTAR) {
        nonGsPartsSums.push(nonGsParts);
        currentBody = [[], 0];
        bodySegments.push(currentBody);
      } else {
        currentBody[0].push(b);
        nonGsParts++;
      }
    }
    let i = bodySegments.length - 1;
    const fileLength = file.length - fileTailMatch;
    for (const b of bodySegments) {
      b[1] = fileLength - (nonGsPartsSums[i--] + b[0].length);
    }
    return !!this.#matchGlobStarBodySections(file, bodySegments, fileIndex, 0, partial, 0, !!fileTailMatch);
  }
  // return false for "nope, not matching"
  // return null for "not matching, cannot keep trying"
  #matchGlobStarBodySections(file, bodySegments, fileIndex, bodyIndex, partial, globStarDepth, sawTail) {
    const bs = bodySegments[bodyIndex];
    if (!bs) {
      for (let i = fileIndex; i < file.length; i++) {
        sawTail = true;
        const f = file[i];
        if (f === "." || f === ".." || !this.options.dot && f.startsWith(".")) {
          return false;
        }
      }
      return sawTail;
    }
    const [body, after] = bs;
    while (fileIndex <= after) {
      const m = this.#matchOne(file.slice(0, fileIndex + body.length), body, partial, fileIndex, 0);
      if (m && globStarDepth < this.maxGlobstarRecursion) {
        const sub = this.#matchGlobStarBodySections(file, bodySegments, fileIndex + body.length, bodyIndex + 1, partial, globStarDepth + 1, sawTail);
        if (sub !== false) {
          return sub;
        }
      }
      const f = file[fileIndex];
      if (f === "." || f === ".." || !this.options.dot && f.startsWith(".")) {
        return false;
      }
      fileIndex++;
    }
    return partial || null;
  }
  #matchOne(file, pattern, partial, fileIndex, patternIndex) {
    let fi;
    let pi;
    let pl;
    let fl;
    for (fi = fileIndex, pi = patternIndex, fl = file.length, pl = pattern.length; fi < fl && pi < pl; fi++, pi++) {
      this.debug("matchOne loop");
      let p = pattern[pi];
      let f = file[fi];
      this.debug(pattern, p, f);
      if (p === false || p === GLOBSTAR) {
        return false;
      }
      let hit;
      if (typeof p === "string") {
        hit = f === p;
        this.debug("string match", p, f, hit);
      } else {
        hit = p.test(f);
        this.debug("pattern match", p, f, hit);
      }
      if (!hit)
        return false;
    }
    if (fi === fl && pi === pl) {
      return true;
    } else if (fi === fl) {
      return partial;
    } else if (pi === pl) {
      return fi === fl - 1 && file[fi] === "";
    } else {
      throw new Error("wtf?");
    }
  }
  braceExpand() {
    return braceExpand(this.pattern, this.options);
  }
  parse(pattern) {
    assertValidPattern(pattern);
    const options = this.options;
    if (pattern === "**")
      return GLOBSTAR;
    if (pattern === "")
      return "";
    let m;
    let fastTest = null;
    if (m = pattern.match(starRE)) {
      fastTest = options.dot ? starTestDot : starTest;
    } else if (m = pattern.match(starDotExtRE)) {
      fastTest = (options.nocase ? options.dot ? starDotExtTestNocaseDot : starDotExtTestNocase : options.dot ? starDotExtTestDot : starDotExtTest)(m[1]);
    } else if (m = pattern.match(qmarksRE)) {
      fastTest = (options.nocase ? options.dot ? qmarksTestNocaseDot : qmarksTestNocase : options.dot ? qmarksTestDot : qmarksTest)(m);
    } else if (m = pattern.match(starDotStarRE)) {
      fastTest = options.dot ? starDotStarTestDot : starDotStarTest;
    } else if (m = pattern.match(dotStarRE)) {
      fastTest = dotStarTest;
    }
    const re = AST.fromGlob(pattern, this.options).toMMPattern();
    if (fastTest && typeof re === "object") {
      Reflect.defineProperty(re, "test", { value: fastTest });
    }
    return re;
  }
  makeRe() {
    if (this.regexp || this.regexp === false)
      return this.regexp;
    const set = this.set;
    if (!set.length) {
      this.regexp = false;
      return this.regexp;
    }
    const options = this.options;
    const twoStar = options.noglobstar ? star2 : options.dot ? twoStarDot : twoStarNoDot;
    const flags = new Set(options.nocase ? ["i"] : []);
    let re = set.map((pattern) => {
      const pp = pattern.map((p) => {
        if (p instanceof RegExp) {
          for (const f of p.flags.split(""))
            flags.add(f);
        }
        return typeof p === "string" ? regExpEscape2(p) : p === GLOBSTAR ? GLOBSTAR : p._src;
      });
      pp.forEach((p, i) => {
        const next = pp[i + 1];
        const prev = pp[i - 1];
        if (p !== GLOBSTAR || prev === GLOBSTAR) {
          return;
        }
        if (prev === void 0) {
          if (next !== void 0 && next !== GLOBSTAR) {
            pp[i + 1] = "(?:\\/|" + twoStar + "\\/)?" + next;
          } else {
            pp[i] = twoStar;
          }
        } else if (next === void 0) {
          pp[i - 1] = prev + "(?:\\/|\\/" + twoStar + ")?";
        } else if (next !== GLOBSTAR) {
          pp[i - 1] = prev + "(?:\\/|\\/" + twoStar + "\\/)" + next;
          pp[i + 1] = GLOBSTAR;
        }
      });
      const filtered = pp.filter((p) => p !== GLOBSTAR);
      if (this.partial && filtered.length >= 1) {
        const prefixes = [];
        for (let i = 1; i <= filtered.length; i++) {
          prefixes.push(filtered.slice(0, i).join("/"));
        }
        return "(?:" + prefixes.join("|") + ")";
      }
      return filtered.join("/");
    }).join("|");
    const [open3, close] = set.length > 1 ? ["(?:", ")"] : ["", ""];
    re = "^" + open3 + re + close + "$";
    if (this.partial) {
      re = "^(?:\\/|" + open3 + re.slice(1, -1) + close + ")$";
    }
    if (this.negate)
      re = "^(?!" + re + ").+$";
    try {
      this.regexp = new RegExp(re, [...flags].join(""));
    } catch {
      this.regexp = false;
    }
    return this.regexp;
  }
  slashSplit(p) {
    if (this.preserveMultipleSlashes) {
      return p.split("/");
    } else if (this.isWindows && /^\/\/[^/]+/.test(p)) {
      return ["", ...p.split(/\/+/)];
    } else {
      return p.split(/\/+/);
    }
  }
  match(f, partial = this.partial) {
    this.debug("match", f, this.pattern);
    if (this.comment) {
      return false;
    }
    if (this.empty) {
      return f === "";
    }
    if (f === "/" && partial) {
      return true;
    }
    const options = this.options;
    if (this.isWindows) {
      f = f.split("\\").join("/");
    }
    const ff = this.slashSplit(f);
    this.debug(this.pattern, "split", ff);
    const set = this.set;
    this.debug(this.pattern, "set", set);
    let filename = ff[ff.length - 1];
    if (!filename) {
      for (let i = ff.length - 2; !filename && i >= 0; i--) {
        filename = ff[i];
      }
    }
    for (const pattern of set) {
      let file = ff;
      if (options.matchBase && pattern.length === 1) {
        file = [filename];
      }
      const hit = this.matchOne(file, pattern, partial);
      if (hit) {
        if (options.flipNegate) {
          return true;
        }
        return !this.negate;
      }
    }
    if (options.flipNegate) {
      return false;
    }
    return this.negate;
  }
  static defaults(def) {
    return minimatch.defaults(def).Minimatch;
  }
};
minimatch.AST = AST;
minimatch.Minimatch = Minimatch;
minimatch.escape = escape;
minimatch.unescape = unescape;

// target/vendor-pi/dist/cli/args.js
var VALID_THINKING_LEVELS = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];
function isValidThinkingLevel(level) {
  return VALID_THINKING_LEVELS.includes(level);
}
function parseArgs(args) {
  const result = {
    messages: [],
    fileArgs: [],
    unknownFlags: /* @__PURE__ */ new Map(),
    diagnostics: []
  };
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === "--") {
      for (const positionalArg of args.slice(i + 1)) {
        if (positionalArg.startsWith("@")) {
          result.fileArgs.push(positionalArg.slice(1));
        } else {
          result.messages.push(positionalArg);
        }
      }
      break;
    } else if (arg === "--help" || arg === "-h") {
      result.help = true;
    } else if (arg === "--version" || arg === "-v") {
      result.version = true;
    } else if (arg === "--mode") {
      const mode = args[i + 1];
      if (mode === void 0 || mode.startsWith("-")) {
        result.diagnostics.push({ type: "error", message: "--mode requires text, json, or rpc" });
        continue;
      }
      i++;
      if (mode !== "text" && mode !== "json" && mode !== "rpc") {
        result.diagnostics.push({
          type: "error",
          message: `Invalid mode "${mode}". Valid values: text, json, rpc`
        });
        continue;
      }
      result.mode = mode;
    } else if (arg === "--continue" || arg === "-c") {
      result.continue = true;
    } else if (arg === "--resume" || arg === "-r") {
      result.resume = true;
    } else if (arg === "--provider" && i + 1 < args.length) {
      result.provider = args[++i];
    } else if (arg === "--model" && i + 1 < args.length) {
      result.model = args[++i];
    } else if (arg === "--api-key" && i + 1 < args.length) {
      result.apiKey = args[++i];
    } else if (arg === "--system-prompt" && i + 1 < args.length) {
      result.systemPrompt = args[++i];
    } else if (arg === "--append-system-prompt" && i + 1 < args.length) {
      result.appendSystemPrompt = result.appendSystemPrompt ?? [];
      result.appendSystemPrompt.push(args[++i]);
    } else if (arg === "--name" || arg === "-n") {
      if (i + 1 < args.length) {
        result.name = args[++i];
      } else {
        result.diagnostics.push({ type: "error", message: "--name requires a value" });
      }
    } else if (arg === "--no-session") {
      result.noSession = true;
    } else if (arg === "--session" && i + 1 < args.length) {
      result.session = args[++i];
    } else if (arg === "--session-id" && i + 1 < args.length) {
      result.sessionId = args[++i];
    } else if (arg === "--fork" && i + 1 < args.length) {
      result.fork = args[++i];
    } else if (arg === "--session-dir" && i + 1 < args.length) {
      result.sessionDir = args[++i];
    } else if (arg === "--models" && i + 1 < args.length) {
      result.models = args[++i].split(",").map((s) => s.trim());
    } else if (arg === "--no-tools" || arg === "-nt") {
      result.noTools = true;
    } else if (arg === "--no-builtin-tools" || arg === "-nbt") {
      result.noBuiltinTools = true;
    } else if ((arg === "--tools" || arg === "-t") && i + 1 < args.length) {
      result.tools = args[++i].split(",").map((s) => s.trim()).filter((name) => name.length > 0);
    } else if ((arg === "--exclude-tools" || arg === "-xt") && i + 1 < args.length) {
      result.excludeTools = args[++i].split(",").map((s) => s.trim()).filter((name) => name.length > 0);
    } else if (arg === "--thinking" && i + 1 < args.length) {
      const level = args[++i];
      if (isValidThinkingLevel(level)) {
        result.thinking = level;
      } else {
        result.diagnostics.push({
          type: "warning",
          message: `Invalid thinking level "${level}". Valid values: ${VALID_THINKING_LEVELS.join(", ")}`
        });
      }
    } else if (arg === "--print" || arg === "-p") {
      result.print = true;
      const next = args[i + 1];
      if (next !== void 0 && !next.startsWith("@") && (!next.startsWith("-") || next.startsWith("---"))) {
        result.messages.push(next);
        i++;
      }
    } else if (arg === "--export" && i + 1 < args.length) {
      result.export = args[++i];
    } else if ((arg === "--extension" || arg === "-e") && i + 1 < args.length) {
      result.extensions = result.extensions ?? [];
      result.extensions.push(args[++i]);
    } else if (arg === "--no-extensions" || arg === "-ne") {
      result.noExtensions = true;
    } else if (arg === "--skill" && i + 1 < args.length) {
      result.skills = result.skills ?? [];
      result.skills.push(args[++i]);
    } else if (arg === "--prompt-template" && i + 1 < args.length) {
      result.promptTemplates = result.promptTemplates ?? [];
      result.promptTemplates.push(args[++i]);
    } else if (arg === "--theme" && i + 1 < args.length) {
      result.themes = result.themes ?? [];
      result.themes.push(args[++i]);
    } else if (arg === "--use-theme") {
      const themeName = args[i + 1];
      if (themeName === void 0 || themeName.startsWith("-")) {
        result.diagnostics.push({ type: "error", message: "--use-theme requires a theme name" });
      } else {
        result.useTheme = themeName;
        i++;
      }
    } else if (arg === "--no-skills" || arg === "-ns") {
      result.noSkills = true;
    } else if (arg === "--no-prompt-templates" || arg === "-np") {
      result.noPromptTemplates = true;
    } else if (arg === "--no-themes") {
      result.noThemes = true;
    } else if (arg === "--no-context-files" || arg === "-nc") {
      result.noContextFiles = true;
    } else if (arg === "--list-models") {
      if (i + 1 < args.length && !args[i + 1].startsWith("-") && !args[i + 1].startsWith("@")) {
        result.listModels = args[++i];
      } else {
        result.listModels = true;
      }
    } else if (arg === "--tui-mode") {
      const mode = args[i + 1];
      if (mode === "regular" || mode === "fullscreen") {
        result.tuiMode = mode;
        i++;
      } else if (mode === void 0 || mode.startsWith("-")) {
        result.diagnostics.push({ type: "error", message: "--tui-mode requires regular or fullscreen" });
      } else {
        i++;
        result.diagnostics.push({
          type: "error",
          message: `Invalid TUI mode "${mode}". Valid values: regular, fullscreen`
        });
      }
    } else if (arg === "--verbose") {
      result.verbose = true;
    } else if (arg === "--approve" || arg === "-a") {
      result.projectTrustOverride = true;
    } else if (arg === "--no-approve" || arg === "-na") {
      result.projectTrustOverride = false;
    } else if (arg === "--offline") {
      result.offline = true;
    } else if (arg.startsWith("@")) {
      result.fileArgs.push(arg.slice(1));
    } else if (arg.startsWith("--")) {
      const eqIndex = arg.indexOf("=");
      if (eqIndex !== -1) {
        result.unknownFlags.set(arg.slice(2, eqIndex), arg.slice(eqIndex + 1));
      } else {
        const flagName = arg.slice(2);
        const next = args[i + 1];
        if (next !== void 0 && !next.startsWith("-") && !next.startsWith("@")) {
          result.unknownFlags.set(flagName, next);
          i++;
        } else {
          result.unknownFlags.set(flagName, true);
        }
      }
    } else if (arg.startsWith("-") && !arg.startsWith("--")) {
      result.diagnostics.push({ type: "error", message: `Unknown option: ${arg}` });
    } else if (!arg.startsWith("-")) {
      result.messages.push(arg);
    }
  }
  return result;
}

// target/vendor-pi/dist/core/model-resolver.js
var defaultModelPerProvider = {
  "amazon-bedrock": "us.anthropic.claude-opus-4-6-v1",
  "ant-ling": "Ring-2.6-1T",
  anthropic: "claude-opus-4-8",
  openai: "gpt-5.5",
  "azure-openai-responses": "gpt-5.4",
  "openai-codex": "gpt-6.1-sol",
  radius: "balanced",
  nvidia: "nvidia/nemotron-3-super-120b-a12b",
  deepseek: "deepseek-v4-pro",
  google: "gemini-3.1-pro-preview",
  "google-vertex": "gemini-3.1-pro-preview",
  "github-copilot": "gpt-5.4",
  openrouter: "moonshotai/kimi-k2.6",
  "vercel-ai-gateway": "zai/glm-5.1",
  xai: "grok-4.7",
  groq: "openai/gpt-oss-120b",
  cerebras: "gpt-oss-120b",
  zai: "glm-5.3",
  "zai-coding-cn": "glm-5.3",
  mistral: "devstral-medium-latest",
  minimax: "MiniMax-M2.7",
  "minimax-cn": "MiniMax-M2.7",
  moonshotai: "kimi-k2.6",
  "moonshotai-cn": "kimi-k2.6",
  huggingface: "moonshotai/Kimi-K2.6",
  fireworks: "accounts/fireworks/models/kimi-k3",
  together: "moonshotai/Kimi-K3",
  baseten: "zai-org/GLM-5.2",
  opencode: "kimi-k2.6",
  "opencode-go": "kimi-k3",
  "kimi-coding": "kimi-for-coding",
  meta: "muse-spark-1.3",
  "cloudflare-workers-ai": "@cf/moonshotai/kimi-k2.6",
  "cloudflare-ai-gateway": "workers-ai/@cf/moonshotai/kimi-k2.6",
  "qwen-token-plan": "qwen3.7-max",
  "qwen-token-plan-cn": "qwen3.7-max",
  "qwen-token-plan-individual": "qwen3.8-max",
  xiaomi: "mimo-v2.5-pro",
  "xiaomi-token-plan-cn": "mimo-v2.5-pro",
  "xiaomi-token-plan-ams": "mimo-v2.5-pro",
  "xiaomi-token-plan-sgp": "mimo-v2.5-pro"
};
function isAlias(id) {
  if (id.endsWith("-latest"))
    return true;
  const datePattern = /-\d{8}$/;
  return !datePattern.test(id);
}
function findExactModelReferenceMatch(modelReference, availableModels) {
  const trimmedReference = modelReference.trim();
  if (!trimmedReference) {
    return void 0;
  }
  const normalizedReference = trimmedReference.toLowerCase();
  const canonicalMatches = availableModels.filter((model) => `${model.provider}/${model.id}`.toLowerCase() === normalizedReference);
  if (canonicalMatches.length === 1) {
    return canonicalMatches[0];
  }
  if (canonicalMatches.length > 1) {
    return void 0;
  }
  const slashIndex = trimmedReference.indexOf("/");
  if (slashIndex !== -1) {
    const provider = trimmedReference.substring(0, slashIndex).trim();
    const modelId = trimmedReference.substring(slashIndex + 1).trim();
    if (provider && modelId) {
      const providerMatches = availableModels.filter((model) => model.provider.toLowerCase() === provider.toLowerCase() && model.id.toLowerCase() === modelId.toLowerCase());
      if (providerMatches.length === 1) {
        return providerMatches[0];
      }
      if (providerMatches.length > 1) {
        return void 0;
      }
    }
  }
  const idMatches = availableModels.filter((model) => model.id.toLowerCase() === normalizedReference);
  return idMatches.length === 1 ? idMatches[0] : void 0;
}
function tryMatchModel(modelPattern, availableModels) {
  const exactMatch = findExactModelReferenceMatch(modelPattern, availableModels);
  if (exactMatch) {
    return exactMatch;
  }
  const matches = availableModels.filter((m) => m.id.toLowerCase().includes(modelPattern.toLowerCase()) || m.name?.toLowerCase().includes(modelPattern.toLowerCase()));
  if (matches.length === 0) {
    return void 0;
  }
  const aliases = matches.filter((m) => isAlias(m.id));
  const datedVersions = matches.filter((m) => !isAlias(m.id));
  if (aliases.length > 0) {
    aliases.sort((a, b) => b.id.localeCompare(a.id));
    return aliases[0];
  } else {
    datedVersions.sort((a, b) => b.id.localeCompare(a.id));
    return datedVersions[0];
  }
}
function buildFallbackModel(provider, modelId, availableModels) {
  const providerModels = availableModels.filter((m) => m.provider === provider);
  if (providerModels.length === 0)
    return void 0;
  const defaultId = defaultModelPerProvider[provider];
  const baseModel = defaultId ? providerModels.find((m) => m.id === defaultId) ?? providerModels[0] : providerModels[0];
  return {
    ...baseModel,
    id: modelId,
    name: modelId
  };
}
function parseModelPattern(pattern, availableModels, options) {
  const exactMatch = tryMatchModel(pattern, availableModels);
  if (exactMatch) {
    return { model: exactMatch, thinkingLevel: void 0, warning: void 0 };
  }
  const lastColonIndex = pattern.lastIndexOf(":");
  if (lastColonIndex === -1) {
    return { model: void 0, thinkingLevel: void 0, warning: void 0 };
  }
  const prefix = pattern.substring(0, lastColonIndex);
  const suffix = pattern.substring(lastColonIndex + 1);
  if (isValidThinkingLevel(suffix)) {
    const result = parseModelPattern(prefix, availableModels, options);
    if (result.model) {
      return {
        model: result.model,
        thinkingLevel: result.warning ? void 0 : suffix,
        warning: result.warning
      };
    }
    return result;
  } else {
    const allowFallback = options?.allowInvalidThinkingLevelFallback ?? true;
    if (!allowFallback) {
      return { model: void 0, thinkingLevel: void 0, warning: void 0 };
    }
    const result = parseModelPattern(prefix, availableModels, options);
    if (result.model) {
      return {
        model: result.model,
        thinkingLevel: void 0,
        warning: `Invalid thinking level "${suffix}" in pattern "${pattern}". Using default instead.`
      };
    }
    return result;
  }
}
function resolveModelScopeFromModels(patterns, models) {
  const availableModels = [...models];
  const scopedModels = [];
  const diagnostics = [];
  for (const pattern of patterns) {
    if (pattern.includes("*") || pattern.includes("?") || pattern.includes("[")) {
      const colonIdx = pattern.lastIndexOf(":");
      let globPattern = pattern;
      let thinkingLevel2;
      if (colonIdx !== -1) {
        const suffix = pattern.substring(colonIdx + 1);
        if (isValidThinkingLevel(suffix)) {
          thinkingLevel2 = suffix;
          globPattern = pattern.substring(0, colonIdx);
        }
      }
      const exactMatch = findExactModelReferenceMatch(globPattern, availableModels);
      if (exactMatch) {
        if (!scopedModels.find((sm) => modelsAreEqual(sm.model, exactMatch))) {
          scopedModels.push({ model: exactMatch, thinkingLevel: thinkingLevel2 });
        }
        continue;
      }
      const matchingModels = availableModels.filter((m) => {
        const fullId = `${m.provider}/${m.id}`;
        return minimatch(fullId, globPattern, { nocase: true }) || minimatch(m.id, globPattern, { nocase: true });
      });
      if (matchingModels.length === 0) {
        diagnostics.push({
          type: "warning",
          code: "no-match",
          message: `No models match pattern "${pattern}"`,
          pattern
        });
        continue;
      }
      for (const model2 of matchingModels) {
        if (!scopedModels.find((sm) => modelsAreEqual(sm.model, model2))) {
          scopedModels.push({ model: model2, thinkingLevel: thinkingLevel2 });
        }
      }
      continue;
    }
    const { model, thinkingLevel, warning } = parseModelPattern(pattern, availableModels);
    if (warning) {
      diagnostics.push({ type: "warning", code: "invalid-thinking-level", message: warning, pattern });
    }
    if (!model) {
      diagnostics.push({
        type: "warning",
        code: "no-match",
        message: `No models match pattern "${pattern}"`,
        pattern
      });
      continue;
    }
    if (!scopedModels.find((sm) => modelsAreEqual(sm.model, model))) {
      scopedModels.push({ model, thinkingLevel });
    }
  }
  return { scopedModels, diagnostics };
}
async function resolveModelScopeWithDiagnostics(patterns, modelRuntime, options) {
  return resolveModelScopeFromModels(patterns, await modelRuntime.getAvailable(void 0, options));
}
function resolveCliModel(options) {
  const { cliProvider, cliModel, cliThinking, modelRuntime } = options;
  if (!cliModel) {
    return { model: void 0, warning: void 0, error: void 0 };
  }
  const availableModels = [...modelRuntime.getModels()];
  if (availableModels.length === 0) {
    return {
      model: void 0,
      warning: void 0,
      error: "No models available. Check your installation or add models to models.json."
    };
  }
  const providerMap = /* @__PURE__ */ new Map();
  for (const m of availableModels) {
    providerMap.set(m.provider.toLowerCase(), m.provider);
  }
  let provider = cliProvider ? providerMap.get(cliProvider.toLowerCase()) : void 0;
  if (cliProvider && !provider) {
    return {
      model: void 0,
      warning: void 0,
      error: `Unknown provider "${cliProvider}". Use --list-models to see available providers/models.`
    };
  }
  let pattern = cliModel;
  let inferredProvider = false;
  if (!provider) {
    const slashIndex = cliModel.indexOf("/");
    if (slashIndex !== -1) {
      const maybeProvider = cliModel.substring(0, slashIndex);
      const canonical = providerMap.get(maybeProvider.toLowerCase());
      if (canonical) {
        provider = canonical;
        pattern = cliModel.substring(slashIndex + 1);
        inferredProvider = true;
      }
    }
  }
  if (!provider) {
    const lower = cliModel.toLowerCase();
    const exactMatches = availableModels.filter((m) => m.id.toLowerCase() === lower || `${m.provider}/${m.id}`.toLowerCase() === lower);
    if (exactMatches.length === 1) {
      return { model: exactMatches[0], warning: void 0, thinkingLevel: void 0, error: void 0 };
    }
    if (exactMatches.length > 1) {
      const authenticatedExactMatches = exactMatches.filter((m) => modelRuntime.hasConfiguredAuth(m.provider));
      if (authenticatedExactMatches.length === 1) {
        return {
          model: authenticatedExactMatches[0],
          warning: void 0,
          thinkingLevel: void 0,
          error: void 0
        };
      }
      const matches = exactMatches.map((m) => `${m.provider}/${m.id}`).sort((a, b) => a.localeCompare(b)).join(", ");
      const authHint = authenticatedExactMatches.length === 0 ? "No matching provider is authenticated." : "More than one matching provider is authenticated.";
      return {
        model: void 0,
        warning: void 0,
        thinkingLevel: void 0,
        error: `Model "${cliModel}" is ambiguous across providers: ${matches}. ${authHint} Use --provider or provider/model.`
      };
    }
  }
  if (cliProvider && provider) {
    const prefix = `${provider}/`;
    if (cliModel.toLowerCase().startsWith(prefix.toLowerCase())) {
      pattern = cliModel.substring(prefix.length);
    }
  }
  const candidates = provider ? availableModels.filter((m) => m.provider === provider) : availableModels;
  const { model, thinkingLevel, warning } = parseModelPattern(pattern, candidates, {
    allowInvalidThinkingLevelFallback: false
  });
  if (model) {
    if (inferredProvider) {
      const rawExactMatches = availableModels.filter((m) => m.id.toLowerCase() === cliModel.toLowerCase() && !modelsAreEqual(m, model));
      if (rawExactMatches.length > 0 && !modelRuntime.hasConfiguredAuth(model.provider)) {
        const authenticatedRawMatches = rawExactMatches.filter((m) => modelRuntime.hasConfiguredAuth(m.provider));
        if (authenticatedRawMatches.length === 1) {
          return {
            model: authenticatedRawMatches[0],
            thinkingLevel: void 0,
            warning: void 0,
            error: void 0
          };
        }
      }
    }
    return { model, thinkingLevel, warning, error: void 0 };
  }
  if (inferredProvider) {
    const lower = cliModel.toLowerCase();
    const exact = availableModels.find((m) => m.id.toLowerCase() === lower || `${m.provider}/${m.id}`.toLowerCase() === lower);
    if (exact) {
      return { model: exact, warning: void 0, thinkingLevel: void 0, error: void 0 };
    }
    const fallback = parseModelPattern(cliModel, availableModels, {
      allowInvalidThinkingLevelFallback: false
    });
    if (fallback.model) {
      return {
        model: fallback.model,
        thinkingLevel: fallback.thinkingLevel,
        warning: fallback.warning,
        error: void 0
      };
    }
  }
  if (provider) {
    let fallbackPattern = pattern;
    let fallbackThinking;
    if (!cliThinking) {
      const lastColon = pattern.lastIndexOf(":");
      if (lastColon !== -1) {
        const suffix = pattern.substring(lastColon + 1);
        if (isValidThinkingLevel(suffix)) {
          fallbackPattern = pattern.substring(0, lastColon);
          fallbackThinking = suffix;
        }
      }
    }
    const fallbackModel = buildFallbackModel(provider, fallbackPattern, availableModels);
    if (fallbackModel) {
      const requestedThinking = cliThinking ?? fallbackThinking;
      const model2 = requestedThinking && requestedThinking !== "off" ? { ...fallbackModel, reasoning: true } : fallbackModel;
      const fallbackWarning = warning ? `${warning} Model "${fallbackPattern}" not found for provider "${provider}". Using custom model id.` : `Model "${fallbackPattern}" not found for provider "${provider}". Using custom model id.`;
      return { model: model2, thinkingLevel: fallbackThinking, warning: fallbackWarning, error: void 0 };
    }
  }
  const display = provider ? `${provider}/${pattern}` : cliModel;
  return {
    model: void 0,
    thinkingLevel: void 0,
    warning,
    error: `Model "${display}" not found. Use --list-models to see available models.`
  };
}

// target/vendor-pi/dist/core/auth-storage.js
var import_proper_lockfile = __toESM(require_proper_lockfile());
import { existsSync as existsSync4, mkdirSync as mkdirSync2, readFileSync as readFileSync2, writeFileSync as writeFileSync2 } from "fs";
import { dirname as dirname2, join as join5 } from "path";
import { setTimeout as sleep } from "timers/promises";

// target/vendor-pi/dist/utils/abort.js
function abortReason(signal) {
  if (signal.reason !== void 0)
    return signal.reason;
  const error = new Error("The operation was aborted");
  error.name = "AbortError";
  return error;
}
function raceWithAbortSignal(operation, signal) {
  if (!signal)
    return operation;
  if (signal.aborted) {
    void operation.catch(() => {
    });
    return Promise.reject(abortReason(signal));
  }
  return new Promise((resolve5, reject) => {
    let settled = false;
    const cleanup = () => signal.removeEventListener("abort", onAbort);
    const onAbort = () => {
      if (settled)
        return;
      settled = true;
      cleanup();
      reject(abortReason(signal));
    };
    signal.addEventListener("abort", onAbort, { once: true });
    void operation.then((value) => {
      if (settled)
        return;
      settled = true;
      cleanup();
      resolve5(value);
    }, (error) => {
      if (settled)
        return;
      settled = true;
      cleanup();
      reject(error);
    });
    if (signal.aborted)
      onAbort();
  });
}

// target/vendor-pi/dist/core/resolve-config-value.js
import { execSync, spawnSync as spawnSync2 } from "child_process";

// target/vendor-pi/dist/utils/shell.js
import { existsSync as existsSync3 } from "node:fs";
import { delimiter, join as join4 } from "node:path";
import { spawn, spawnSync } from "child_process";
function isLegacyWslBashPath(path4) {
  const normalized = path4.replace(/\//g, "\\").toLowerCase();
  return /^[a-z]:\\windows\\(?:system32|sysnative)\\bash\.exe$/.test(normalized);
}
function getBashShellConfig(shell) {
  return isLegacyWslBashPath(shell) ? { shell, args: ["-s"], commandTransport: "stdin" } : { shell, args: ["-c"] };
}
function findExecutableOnPath(executable) {
  if (process.platform === "win32") {
    try {
      const result = spawnSync("where", [executable], {
        encoding: "utf-8",
        timeout: 5e3,
        windowsHide: true
      });
      if (result.status === 0 && result.stdout) {
        const firstMatch = result.stdout.trim().split(/\r?\n/)[0];
        if (firstMatch && existsSync3(firstMatch)) {
          return firstMatch;
        }
      }
    } catch {
    }
    return null;
  }
  try {
    const result = spawnSync("which", [executable], { encoding: "utf-8", timeout: 5e3 });
    if (result.status === 0 && result.stdout) {
      const firstMatch = result.stdout.trim().split(/\r?\n/)[0];
      if (firstMatch) {
        return firstMatch;
      }
    }
  } catch {
  }
  return null;
}
function getShellConfig(customShellPath) {
  if (customShellPath) {
    if (existsSync3(customShellPath)) {
      return getBashShellConfig(customShellPath);
    }
    throw new Error(`Custom shell path not found: ${customShellPath}`);
  }
  if (process.platform === "win32") {
    const paths = [];
    const programFiles = process.env.ProgramFiles;
    if (programFiles) {
      paths.push(`${programFiles}\\Git\\bin\\bash.exe`);
    }
    const programFilesX86 = process.env["ProgramFiles(x86)"];
    if (programFilesX86) {
      paths.push(`${programFilesX86}\\Git\\bin\\bash.exe`);
    }
    for (const path4 of paths) {
      if (existsSync3(path4)) {
        return getBashShellConfig(path4);
      }
    }
    const bashOnPath2 = findExecutableOnPath("bash.exe");
    if (bashOnPath2) {
      return getBashShellConfig(bashOnPath2);
    }
    throw new Error(`No bash shell found. Options:
  1. Install Git for Windows: https://git-scm.com/download/win
  2. Add your bash to PATH (Cygwin, MSYS2, etc.)
  3. Set shellPath in settings.json

Searched Git Bash in:
${paths.map((p) => `  ${p}`).join("\n")}`);
  }
  if (existsSync3("/bin/bash")) {
    return getBashShellConfig("/bin/bash");
  }
  const bashOnPath = findExecutableOnPath("bash");
  if (bashOnPath) {
    return getBashShellConfig(bashOnPath);
  }
  return { shell: "sh", args: ["-c"] };
}
var POWERSHELL_ARGS = ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command"];
function getPowerShellConfig() {
  if (process.platform !== "win32") {
    throw new Error("The powershell tool is only available on Windows.");
  }
  const shell = findExecutableOnPath("pwsh.exe") ?? findExecutableOnPath("powershell.exe");
  if (!shell) {
    throw new Error("No PowerShell executable found. Install PowerShell or add powershell.exe/pwsh.exe to PATH.");
  }
  return { shell, args: [...POWERSHELL_ARGS] };
}
function getShellEnv() {
  const binDir = (0, config_exports.getBinDir)();
  const pathKey = Object.keys(process.env).find((key) => key.toLowerCase() === "path") ?? "PATH";
  const currentPath = process.env[pathKey] ?? "";
  const pathEntries = currentPath.split(delimiter).filter(Boolean);
  const hasBinDir = pathEntries.includes(binDir);
  const updatedPath = hasBinDir ? currentPath : [binDir, currentPath].filter(Boolean).join(delimiter);
  return {
    ...process.env,
    [pathKey]: updatedPath
  };
}
function sanitizeBinaryOutput(str2) {
  return str2.replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\uFFF9-\uFFFB]/g, "");
}
var trackedDetachedChildPids = /* @__PURE__ */ new Set();
function trackDetachedChildPid(pid) {
  trackedDetachedChildPids.add(pid);
}
function untrackDetachedChildPid(pid) {
  trackedDetachedChildPids.delete(pid);
}
function killProcessTree(pid) {
  if (process.platform === "win32") {
    try {
      const child = spawn(join4(process.env.SystemRoot ?? "C:\\Windows", "System32", "taskkill.exe"), ["/F", "/T", "/PID", String(pid)], {
        stdio: "ignore",
        detached: true,
        windowsHide: true
      });
      child.once("error", () => {
      });
    } catch {
    }
  } else {
    try {
      process.kill(-pid, "SIGKILL");
    } catch {
      try {
        process.kill(pid, "SIGKILL");
      } catch {
      }
    }
  }
}

// target/vendor-pi/dist/core/auth-storage.js
function readStoredCredential(providerId, authPath = join5((0, config_exports.getAgentDir)(), "auth.json")) {
  try {
    const data = JSON.parse(stripBom(readFileSync2(normalizePath(authPath), "utf-8")));
    return data[providerId];
  } catch {
    return void 0;
  }
}

// target/vendor-pi/dist/core/trust-manager.js
var import_proper_lockfile2 = __toESM(require_proper_lockfile());
import { existsSync as existsSync5, mkdirSync as mkdirSync3, readFileSync as readFileSync3, writeFileSync as writeFileSync3 } from "node:fs";
import { homedir as homedir2 } from "node:os";
import { dirname as dirname3, join as join6 } from "node:path";
var TRUST_REQUIRING_PROJECT_CONFIG_RESOURCES = [
  "settings.json",
  "mcp.json",
  "extensions",
  "skills",
  "prompts",
  "themes",
  "SYSTEM.md",
  "APPEND_SYSTEM.md"
];
function normalizeCwd(cwd) {
  return canonicalizePath(resolvePath(cwd));
}
function findNearestTrustEntry(data, cwd) {
  let currentDir = normalizeCwd(cwd);
  while (true) {
    const value = data[currentDir];
    if (value === true || value === false) {
      return { path: currentDir, decision: value };
    }
    const parentDir = dirname3(currentDir);
    if (parentDir === currentDir) {
      return null;
    }
    currentDir = parentDir;
  }
}
function readTrustFile(path4) {
  if (!existsSync5(path4)) {
    return {};
  }
  let parsed;
  try {
    parsed = JSON.parse(stripBom(readFileSync3(path4, "utf-8")));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to read trust store ${path4}: ${message}`);
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error(`Invalid trust store ${path4}: expected an object`);
  }
  const data = {};
  for (const [key, value] of Object.entries(parsed)) {
    if (value !== true && value !== false && value !== null) {
      throw new Error(`Invalid trust store ${path4}: value for ${JSON.stringify(key)} must be true, false, or null`);
    }
    data[key] = value;
  }
  return data;
}
function writeTrustFile(path4, data) {
  const sorted = {};
  for (const key of Object.keys(data).sort()) {
    const value = data[key];
    if (value === true || value === false || value === null) {
      sorted[key] = value;
    }
  }
  mkdirSync3(dirname3(path4), { recursive: true });
  writeFileSync3(path4, `${JSON.stringify(sorted, null, 2)}
`, "utf-8");
}
function acquireTrustLockSync(path4) {
  const trustDir = dirname3(path4);
  mkdirSync3(trustDir, { recursive: true });
  const maxAttempts = 10;
  const delayMs = 20;
  let lastError;
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return import_proper_lockfile2.default.lockSync(trustDir, { realpath: false, lockfilePath: `${path4}.lock` });
    } catch (error) {
      const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : void 0;
      if (code !== "ELOCKED" || attempt === maxAttempts) {
        throw error;
      }
      lastError = error;
      const start = Date.now();
      while (Date.now() - start < delayMs) {
      }
    }
  }
  if (lastError instanceof Error) {
    throw lastError;
  }
  throw new Error("Failed to acquire trust store lock");
}
function withTrustFileLock(path4, fn) {
  const release = acquireTrustLockSync(path4);
  try {
    return fn();
  } finally {
    release();
  }
}
function hasTrustRequiringProjectResources(cwd) {
  const homeDir = canonicalizePath(resolvePath(process.env.HOME || homedir2()));
  const userAgentsSkillsDir = join6(homeDir, ".agents", "skills");
  let currentDir = canonicalizePath(resolvePath(cwd));
  const configDir = join6(currentDir, config_exports.CONFIG_DIR_NAME);
  if (TRUST_REQUIRING_PROJECT_CONFIG_RESOURCES.some((entry) => existsSync5(join6(configDir, entry)))) {
    return true;
  }
  while (true) {
    const agentsSkillsDir = join6(currentDir, ".agents", "skills");
    if (agentsSkillsDir !== userAgentsSkillsDir && existsSync5(agentsSkillsDir)) {
      return true;
    }
    const parentDir = dirname3(currentDir);
    if (parentDir === currentDir) {
      return false;
    }
    currentDir = parentDir;
  }
}
var ProjectTrustStore = class {
  trustPath;
  constructor(agentDir) {
    this.trustPath = join6(resolvePath(agentDir), "trust.json");
  }
  get(cwd) {
    return this.getEntry(cwd)?.decision ?? null;
  }
  getEntry(cwd) {
    return withTrustFileLock(this.trustPath, () => {
      const data = readTrustFile(this.trustPath);
      return findNearestTrustEntry(data, cwd);
    });
  }
  set(cwd, decision) {
    this.setMany([{ path: cwd, decision }]);
  }
  setMany(decisions) {
    withTrustFileLock(this.trustPath, () => {
      const data = readTrustFile(this.trustPath);
      for (const { path: path4, decision } of decisions) {
        const key = normalizeCwd(path4);
        if (decision === null) {
          delete data[key];
        } else {
          data[key] = decision;
        }
      }
      writeTrustFile(this.trustPath, data);
    });
  }
};

// target/vendor-pi/dist/core/settings-manager.js
var import_proper_lockfile3 = __toESM(require_proper_lockfile());
import { DEFAULT_MAX_AGENT_RETRY_DELAY_MS } from "@earendil-works/pi-ai";
import { randomUUID as randomUUID2 } from "crypto";
import { existsSync as existsSync6, mkdirSync as mkdirSync4, readFileSync as readFileSync4, writeFileSync as writeFileSync4 } from "fs";
import { dirname as dirname4, join as join7 } from "path";

// target/vendor-pi/dist/core/http-dispatcher.js
import { EventEmitter } from "node:events";
var DEFAULT_HTTP_IDLE_TIMEOUT_MS = 3e5;
var HTTP_IDLE_TIMEOUT_CHOICES = [
  { label: "30 sec", timeoutMs: 3e4 },
  { label: "1 min", timeoutMs: 6e4 },
  { label: "2 min", timeoutMs: 12e4 },
  { label: "5 min", timeoutMs: 3e5 },
  { label: "disabled", timeoutMs: 0 }
];
var originalGlobalFetch = globalThis.fetch;
function parseHttpIdleTimeoutMs(value) {
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (trimmed.toLowerCase() === "disabled") {
      return 0;
    }
    if (trimmed.length === 0) {
      return void 0;
    }
    return parseHttpIdleTimeoutMs(Number(trimmed));
  }
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    return void 0;
  }
  return Math.floor(value);
}
function formatHttpIdleTimeoutMs(timeoutMs) {
  const choice = HTTP_IDLE_TIMEOUT_CHOICES.find((item) => item.timeoutMs === timeoutMs);
  if (choice) {
    return choice.label;
  }
  return `${timeoutMs / 1e3} sec`;
}

// target/vendor-pi/dist/core/settings-manager.js
var DEFAULT_COMPACTION_TOKEN_SETTINGS = {
  reserveTokens: 16384,
  keepRecentTokens: 2e4
};
var CACHE_WARMING_MODES = ["off", "streaming", "idle"];
function isMergeableObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function deepMergeObjects(base, overrides) {
  const result = { ...base };
  for (const key of Object.keys(overrides)) {
    const overrideValue = overrides[key];
    if (overrideValue === void 0) {
      continue;
    }
    const baseValue = base[key];
    result[key] = isMergeableObject(baseValue) && isMergeableObject(overrideValue) ? deepMergeObjects(baseValue, overrideValue) : overrideValue;
  }
  return result;
}
var DEFAULT_TOOL_NAMES = ["read", "bash", "edit", "write"];
function isToolModifier(entry) {
  return typeof entry === "string" && (entry.startsWith("+") || entry.startsWith("-"));
}
function mergeDefaultTools(base, overrides) {
  if (overrides === void 0)
    return base;
  if (!Array.isArray(base) || !Array.isArray(overrides) || !overrides.every(isToolModifier))
    return overrides;
  return [...base, ...overrides];
}
function resolveDefaultTools(entries) {
  const plain = entries.filter((entry) => !isToolModifier(entry));
  const tools = plain.length > 0 || entries.length === 0 ? plain : [...DEFAULT_TOOL_NAMES];
  for (const entry of entries) {
    if (!isToolModifier(entry))
      continue;
    const name = entry.slice(1);
    const index = tools.indexOf(name);
    if (entry.startsWith("+") && index === -1 && name)
      tools.push(name);
    else if (entry.startsWith("-") && index !== -1)
      tools.splice(index, 1);
  }
  return tools;
}
function deepMergeSettings(base, overrides) {
  const merged = deepMergeObjects(base, overrides);
  const defaultTools = mergeDefaultTools(base.defaultTools, overrides.defaultTools);
  return defaultTools === void 0 ? merged : { ...merged, defaultTools };
}
function parseTimeoutSetting(value, settingName) {
  const timeoutMs = parseHttpIdleTimeoutMs(value);
  if (timeoutMs !== void 0) {
    return timeoutMs;
  }
  if (value !== void 0) {
    throw new Error(`Invalid ${settingName} setting: ${String(value)}`);
  }
  return void 0;
}
function toSettingsError(scope, error, path4) {
  return {
    scope,
    ...path4 ? { path: path4 } : {},
    error: error instanceof Error ? error : new Error(String(error))
  };
}
var FileSettingsStorage = class {
  globalSettingsPath;
  projectSettingsPath;
  constructor(cwd, agentDir) {
    const resolvedCwd = resolvePath(cwd);
    const resolvedAgentDir = resolvePath(agentDir);
    this.globalSettingsPath = join7(resolvedAgentDir, "settings.json");
    this.projectSettingsPath = join7(resolvedCwd, config_exports.CONFIG_DIR_NAME, "settings.json");
  }
  acquireLockSyncWithRetry(path4) {
    const maxAttempts = 10;
    const delayMs = 20;
    let lastError;
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        return import_proper_lockfile3.default.lockSync(path4, { realpath: false });
      } catch (error) {
        const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : void 0;
        if (code !== "ELOCKED" || attempt === maxAttempts) {
          throw error;
        }
        lastError = error;
        const start = Date.now();
        while (Date.now() - start < delayMs) {
        }
      }
    }
    throw lastError ?? new Error("Failed to acquire settings lock");
  }
  withLock(scope, fn) {
    const path4 = scope === "global" ? this.globalSettingsPath : this.projectSettingsPath;
    const dir = dirname4(path4);
    let release;
    try {
      const fileExists = existsSync6(path4);
      if (fileExists) {
        release = this.acquireLockSyncWithRetry(path4);
      }
      const current = fileExists ? readFileSync4(path4, "utf-8") : void 0;
      const next = fn(current);
      if (next !== void 0) {
        if (!existsSync6(dir)) {
          mkdirSync4(dir, { recursive: true });
        }
        if (!release) {
          release = this.acquireLockSyncWithRetry(path4);
        }
        writeFileSync4(path4, next, "utf-8");
      }
    } finally {
      if (release) {
        release();
      }
    }
  }
};
var InMemorySettingsStorage = class {
  global;
  project;
  withLock(scope, fn) {
    const current = scope === "global" ? this.global : this.project;
    const next = fn(current);
    if (next !== void 0) {
      if (scope === "global") {
        this.global = next;
      } else {
        this.project = next;
      }
    }
  }
};
var SettingsManager = class _SettingsManager {
  storage;
  globalSettings;
  projectSettings;
  settings;
  projectTrusted;
  modifiedFields = /* @__PURE__ */ new Set();
  // Track global fields modified during session
  modifiedNestedFields = /* @__PURE__ */ new Map();
  // Track global nested field modifications
  modifiedProjectFields = /* @__PURE__ */ new Set();
  // Track project fields modified during session
  modifiedProjectNestedFields = /* @__PURE__ */ new Map();
  // Track project nested field modifications
  globalSettingsLoadError = null;
  // Track if global settings file had parse errors
  projectSettingsLoadError = null;
  // Track if project settings file had parse errors
  writeQueue = Promise.resolve();
  errors;
  settingsPaths;
  constructor(storage, initialGlobal, initialProject, globalLoadError = null, projectLoadError = null, initialErrors = [], projectTrusted = true, settingsPaths = {}) {
    this.storage = storage;
    this.globalSettings = initialGlobal;
    this.projectSettings = initialProject;
    this.projectTrusted = projectTrusted;
    this.globalSettingsLoadError = globalLoadError;
    this.projectSettingsLoadError = projectLoadError;
    this.errors = [...initialErrors];
    this.settingsPaths = settingsPaths;
    this.settings = deepMergeSettings(this.globalSettings, this.projectSettings);
  }
  /** Create a SettingsManager that loads from files */
  static create(cwd, agentDir = (0, config_exports.getAgentDir)(), options = {}) {
    const resolvedCwd = resolvePath(cwd);
    const resolvedAgentDir = resolvePath(agentDir);
    const storage = new FileSettingsStorage(resolvedCwd, resolvedAgentDir);
    return _SettingsManager.fromStorageWithPaths(storage, options, {
      global: join7(resolvedAgentDir, "settings.json"),
      project: join7(resolvedCwd, config_exports.CONFIG_DIR_NAME, "settings.json")
    });
  }
  /** Create a SettingsManager from an arbitrary storage backend */
  static fromStorage(storage, options = {}) {
    return _SettingsManager.fromStorageWithPaths(storage, options);
  }
  /** Create a manager while retaining optional file paths for reported storage errors. */
  static fromStorageWithPaths(storage, options, settingsPaths = {}) {
    const projectTrusted = options.projectTrusted ?? true;
    const globalLoad = _SettingsManager.tryLoadFromStorage(storage, "global");
    const projectLoad = _SettingsManager.tryLoadFromStorage(storage, "project", projectTrusted);
    const initialErrors = [];
    if (globalLoad.error) {
      initialErrors.push(toSettingsError("global", globalLoad.error, settingsPaths.global));
    }
    if (projectLoad.error) {
      initialErrors.push(toSettingsError("project", projectLoad.error, settingsPaths.project));
    }
    return new _SettingsManager(storage, globalLoad.settings, projectLoad.settings, globalLoad.error, projectLoad.error, initialErrors, projectTrusted, settingsPaths);
  }
  /** Create an in-memory SettingsManager (no file I/O) */
  static inMemory(settings = {}, options = {}) {
    const storage = new InMemorySettingsStorage();
    const initialSettings = _SettingsManager.migrateSettings(structuredClone(settings));
    storage.withLock("global", () => JSON.stringify(initialSettings, null, 2));
    return _SettingsManager.fromStorage(storage, options);
  }
  static loadFromStorage(storage, scope, projectTrusted = true) {
    if (scope === "project" && !projectTrusted) {
      return {};
    }
    let content;
    storage.withLock(scope, (current) => {
      content = current;
      return void 0;
    });
    if (!content) {
      return {};
    }
    const settings = JSON.parse(stripBom(content));
    return _SettingsManager.migrateSettings(settings);
  }
  static tryLoadFromStorage(storage, scope, projectTrusted = true) {
    try {
      return { settings: _SettingsManager.loadFromStorage(storage, scope, projectTrusted), error: null };
    } catch (error) {
      return { settings: {}, error };
    }
  }
  /** Migrate old settings format to new format */
  static migrateSettings(settings) {
    if ("queueMode" in settings && !("steeringMode" in settings)) {
      settings.steeringMode = settings.queueMode;
      delete settings.queueMode;
    }
    if (!("transport" in settings) && typeof settings.websockets === "boolean") {
      settings.transport = settings.websockets ? "websocket" : "sse";
      delete settings.websockets;
    }
    if ("skills" in settings && typeof settings.skills === "object" && settings.skills !== null && !Array.isArray(settings.skills)) {
      const skillsSettings = settings.skills;
      if (skillsSettings.enableSkillCommands !== void 0 && settings.enableSkillCommands === void 0) {
        settings.enableSkillCommands = skillsSettings.enableSkillCommands;
      }
      if (Array.isArray(skillsSettings.customDirectories) && skillsSettings.customDirectories.length > 0) {
        settings.skills = skillsSettings.customDirectories;
      } else {
        delete settings.skills;
      }
    }
    if ("retry" in settings && typeof settings.retry === "object" && settings.retry !== null && !Array.isArray(settings.retry)) {
      const retrySettings = settings.retry;
      const providerSettings = typeof retrySettings.provider === "object" && retrySettings.provider !== null ? retrySettings.provider : void 0;
      if (typeof retrySettings.maxDelayMs === "number" && (providerSettings?.maxRetryDelayMs === void 0 || providerSettings?.maxRetryDelayMs === null)) {
        retrySettings.provider = {
          ...providerSettings ?? {},
          maxRetryDelayMs: retrySettings.maxDelayMs
        };
      }
      delete retrySettings.maxDelayMs;
    }
    return settings;
  }
  /** A copy of the effective settings: global and project settings merged, with overrides. */
  getSettings() {
    return structuredClone(this.settings);
  }
  getGlobalSettings() {
    return structuredClone(this.globalSettings);
  }
  getProjectSettings() {
    return structuredClone(this.projectSettings);
  }
  isProjectTrusted() {
    return this.projectTrusted;
  }
  setProjectTrusted(trusted) {
    if (this.projectTrusted === trusted) {
      return;
    }
    this.projectTrusted = trusted;
    this.modifiedProjectFields.clear();
    this.modifiedProjectNestedFields.clear();
    if (!trusted) {
      this.projectSettings = {};
      this.projectSettingsLoadError = null;
      this.settings = deepMergeSettings(this.globalSettings, this.projectSettings);
      return;
    }
    const projectLoad = _SettingsManager.tryLoadFromStorage(this.storage, "project", trusted);
    this.projectSettings = projectLoad.settings;
    this.projectSettingsLoadError = projectLoad.error;
    if (projectLoad.error) {
      this.recordError("project", projectLoad.error);
    }
    this.settings = deepMergeSettings(this.globalSettings, this.projectSettings);
  }
  async reload() {
    await this.writeQueue;
    const globalLoad = _SettingsManager.tryLoadFromStorage(this.storage, "global");
    if (!globalLoad.error) {
      this.globalSettings = globalLoad.settings;
      this.globalSettingsLoadError = null;
    } else {
      this.globalSettingsLoadError = globalLoad.error;
      this.recordError("global", globalLoad.error);
    }
    this.modifiedFields.clear();
    this.modifiedNestedFields.clear();
    this.modifiedProjectFields.clear();
    this.modifiedProjectNestedFields.clear();
    const projectLoad = _SettingsManager.tryLoadFromStorage(this.storage, "project", this.projectTrusted);
    if (!projectLoad.error) {
      this.projectSettings = projectLoad.settings;
      this.projectSettingsLoadError = null;
    } else {
      this.projectSettingsLoadError = projectLoad.error;
      this.recordError("project", projectLoad.error);
    }
    this.settings = deepMergeSettings(this.globalSettings, this.projectSettings);
  }
  /** Apply additional overrides on top of current settings */
  applyOverrides(overrides) {
    this.settings = deepMergeSettings(this.settings, overrides);
  }
  /** Mark a global field as modified during this session */
  markModified(field, nestedKey) {
    this.modifiedFields.add(field);
    if (nestedKey) {
      if (!this.modifiedNestedFields.has(field)) {
        this.modifiedNestedFields.set(field, /* @__PURE__ */ new Set());
      }
      this.modifiedNestedFields.get(field).add(nestedKey);
    }
  }
  /** Mark a project field as modified during this session */
  markProjectModified(field, nestedKey) {
    this.modifiedProjectFields.add(field);
    if (nestedKey) {
      if (!this.modifiedProjectNestedFields.has(field)) {
        this.modifiedProjectNestedFields.set(field, /* @__PURE__ */ new Set());
      }
      this.modifiedProjectNestedFields.get(field).add(nestedKey);
    }
  }
  assertProjectTrustedForWrite() {
    if (!this.projectTrusted) {
      throw new Error("Project is not trusted; refusing to write project settings");
    }
  }
  recordError(scope, error) {
    this.errors.push(toSettingsError(scope, error, this.settingsPaths[scope]));
  }
  clearModifiedScope(scope) {
    if (scope === "global") {
      this.modifiedFields.clear();
      this.modifiedNestedFields.clear();
      return;
    }
    this.modifiedProjectFields.clear();
    this.modifiedProjectNestedFields.clear();
  }
  enqueueWrite(scope, task) {
    this.writeQueue = this.writeQueue.then(() => {
      if (scope === "project") {
        this.assertProjectTrustedForWrite();
      }
      task();
      this.clearModifiedScope(scope);
    }).catch((error) => {
      this.recordError(scope, error);
    });
  }
  cloneModifiedNestedFields(source) {
    const snapshot = /* @__PURE__ */ new Map();
    for (const [key, value] of source.entries()) {
      snapshot.set(key, new Set(value));
    }
    return snapshot;
  }
  persistScopedSettings(scope, snapshotSettings, modifiedFields, modifiedNestedFields) {
    this.storage.withLock(scope, (current) => {
      const currentFileSettings = current ? _SettingsManager.migrateSettings(JSON.parse(stripBom(current))) : {};
      const mergedSettings = { ...currentFileSettings };
      for (const field of modifiedFields) {
        const value = snapshotSettings[field];
        if (modifiedNestedFields.has(field) && typeof value === "object" && value !== null) {
          const nestedModified = modifiedNestedFields.get(field);
          const baseNested = currentFileSettings[field] ?? {};
          const inMemoryNested = value;
          const mergedNested = { ...baseNested };
          for (const nestedKey of nestedModified) {
            mergedNested[nestedKey] = inMemoryNested[nestedKey];
          }
          mergedSettings[field] = mergedNested;
        } else {
          mergedSettings[field] = value;
        }
      }
      return JSON.stringify(mergedSettings, null, 2);
    });
  }
  save() {
    this.settings = deepMergeSettings(this.globalSettings, this.projectSettings);
    if (this.globalSettingsLoadError) {
      return;
    }
    const snapshotGlobalSettings = structuredClone(this.globalSettings);
    const modifiedFields = new Set(this.modifiedFields);
    const modifiedNestedFields = this.cloneModifiedNestedFields(this.modifiedNestedFields);
    this.enqueueWrite("global", () => {
      this.persistScopedSettings("global", snapshotGlobalSettings, modifiedFields, modifiedNestedFields);
    });
  }
  saveProjectSettings(settings) {
    this.assertProjectTrustedForWrite();
    this.projectSettings = structuredClone(settings);
    this.settings = deepMergeSettings(this.globalSettings, this.projectSettings);
    if (this.projectSettingsLoadError) {
      return;
    }
    const snapshotProjectSettings = structuredClone(this.projectSettings);
    const modifiedFields = new Set(this.modifiedProjectFields);
    const modifiedNestedFields = this.cloneModifiedNestedFields(this.modifiedProjectNestedFields);
    this.enqueueWrite("project", () => {
      this.persistScopedSettings("project", snapshotProjectSettings, modifiedFields, modifiedNestedFields);
    });
  }
  updateProjectSettings(field, update) {
    this.assertProjectTrustedForWrite();
    const projectSettings = structuredClone(this.projectSettings);
    update(projectSettings);
    this.markProjectModified(field);
    this.saveProjectSettings(projectSettings);
  }
  async flush() {
    await this.writeQueue;
  }
  drainErrors() {
    const drained = [...this.errors];
    this.errors = [];
    return drained;
  }
  getLastChangelogVersion() {
    return this.settings.lastChangelogVersion;
  }
  setLastChangelogVersion(version) {
    this.globalSettings.lastChangelogVersion = version;
    this.markModified("lastChangelogVersion");
    this.save();
  }
  getSessionDir() {
    const sessionDir = this.settings.sessionDir;
    return sessionDir ? normalizePath(sessionDir) : sessionDir;
  }
  getDefaultProvider() {
    return this.settings.defaultProvider;
  }
  getDefaultModel() {
    return this.settings.defaultModel;
  }
  setDefaultProvider(provider) {
    this.globalSettings.defaultProvider = provider;
    this.markModified("defaultProvider");
    this.save();
  }
  setDefaultModel(modelId) {
    this.globalSettings.defaultModel = modelId;
    this.markModified("defaultModel");
    this.save();
  }
  setDefaultModelAndProvider(provider, modelId) {
    this.globalSettings.defaultProvider = provider;
    this.globalSettings.defaultModel = modelId;
    this.markModified("defaultProvider");
    this.markModified("defaultModel");
    this.save();
  }
  getSteeringMode() {
    return this.settings.steeringMode || "one-at-a-time";
  }
  setSteeringMode(mode) {
    this.globalSettings.steeringMode = mode;
    this.markModified("steeringMode");
    this.save();
  }
  getFollowUpMode() {
    return this.settings.followUpMode || "one-at-a-time";
  }
  setFollowUpMode(mode) {
    this.globalSettings.followUpMode = mode;
    this.markModified("followUpMode");
    this.save();
  }
  getThemeSetting() {
    const value = this.settings.theme;
    if (typeof value === "string")
      return value;
    return void 0;
  }
  getTheme() {
    const theme31 = this.getThemeSetting();
    return theme31?.includes("/") ? void 0 : theme31;
  }
  setTheme(theme31) {
    this.globalSettings.theme = theme31;
    this.markModified("theme");
    this.save();
  }
  getDefaultThinkingLevel() {
    return this.settings.defaultThinkingLevel;
  }
  setDefaultThinkingLevel(level) {
    this.globalSettings.defaultThinkingLevel = level;
    this.markModified("defaultThinkingLevel");
    this.save();
  }
  getModelThinkingLevel(provider, modelId) {
    return this.settings.modelThinkingLevels?.[`${provider}/${modelId}`];
  }
  getAllModelThinkingLevels() {
    return { ...this.settings.modelThinkingLevels ?? {} };
  }
  setModelThinkingLevel(provider, modelId, level) {
    if (!this.globalSettings.modelThinkingLevels) {
      this.globalSettings.modelThinkingLevels = {};
    }
    this.globalSettings.modelThinkingLevels[`${provider}/${modelId}`] = level;
    this.markModified("modelThinkingLevels");
    this.save();
  }
  removeModelThinkingLevel(provider, modelId) {
    if (!this.globalSettings.modelThinkingLevels)
      return;
    delete this.globalSettings.modelThinkingLevels[`${provider}/${modelId}`];
    if (Object.keys(this.globalSettings.modelThinkingLevels).length === 0) {
      delete this.globalSettings.modelThinkingLevels;
    }
    this.markModified("modelThinkingLevels");
    this.save();
  }
  getTransport() {
    return this.settings.transport ?? "auto";
  }
  setTransport(transport) {
    this.globalSettings.transport = transport;
    this.markModified("transport");
    this.save();
  }
  getCompactionEnabled() {
    return this.settings.compaction?.enabled ?? true;
  }
  setCompactionEnabled(enabled) {
    if (!this.globalSettings.compaction) {
      this.globalSettings.compaction = {};
    }
    this.globalSettings.compaction.enabled = enabled;
    this.markModified("compaction", "enabled");
    this.save();
  }
  getCompactionTokenSetting(field, model) {
    const compaction = this.settings.compaction;
    const ordinary = compaction?.[field];
    if (ordinary !== void 0 && (typeof ordinary !== "number" || !Number.isSafeInteger(ordinary) || ordinary < 0)) {
      throw new Error(`Invalid compaction.${field} setting: ${String(ordinary)}. Expected a non-negative safe integer.`);
    }
    const modelKey = model ? `${model.provider}/${model.id}` : void 0;
    const entry = modelKey !== void 0 ? compaction?.modelOverrides?.[modelKey] : void 0;
    if (entry !== void 0 && !isMergeableObject(entry)) {
      throw new Error(`Invalid compaction.modelOverrides["${modelKey}"] setting: ${String(entry)}. Expected an object.`);
    }
    const override = entry?.[field];
    if (override !== void 0 && (typeof override !== "number" || !Number.isSafeInteger(override) || override < 0)) {
      throw new Error(`Invalid compaction.modelOverrides["${modelKey}"].${field} setting: ${String(override)}. Expected a non-negative safe integer.`);
    }
    return override ?? ordinary ?? DEFAULT_COMPACTION_TOKEN_SETTINGS[field];
  }
  getCompactionReserveTokens(model) {
    return this.getCompactionTokenSetting("reserveTokens", model);
  }
  getCompactionKeepRecentTokens(model) {
    return this.getCompactionTokenSetting("keepRecentTokens", model);
  }
  /** Resolve each token setting through model override, ordinary setting, then built-in default. */
  getCompactionSettings(model) {
    return {
      enabled: this.getCompactionEnabled(),
      reserveTokens: this.getCompactionReserveTokens(model),
      keepRecentTokens: this.getCompactionKeepRecentTokens(model)
    };
  }
  getBranchSummarySettings() {
    return {
      reserveTokens: this.settings.branchSummary?.reserveTokens ?? 16384,
      skipPrompt: this.settings.branchSummary?.skipPrompt ?? false
    };
  }
  getBranchSummarySkipPrompt() {
    return this.settings.branchSummary?.skipPrompt ?? false;
  }
  getRetryEnabled() {
    return this.settings.retry?.enabled ?? true;
  }
  setRetryEnabled(enabled) {
    if (!this.globalSettings.retry) {
      this.globalSettings.retry = {};
    }
    this.globalSettings.retry.enabled = enabled;
    this.markModified("retry", "enabled");
    this.save();
  }
  getRetrySettings() {
    return {
      enabled: this.getRetryEnabled(),
      maxRetries: this.settings.retry?.maxRetries ?? 3,
      baseDelayMs: this.settings.retry?.baseDelayMs ?? 2e3,
      maxAgentDelayMs: this.settings.retry?.maxAgentDelayMs ?? DEFAULT_MAX_AGENT_RETRY_DELAY_MS
    };
  }
  getHttpIdleTimeoutMs() {
    return parseTimeoutSetting(this.settings.httpIdleTimeoutMs, "httpIdleTimeoutMs") ?? DEFAULT_HTTP_IDLE_TIMEOUT_MS;
  }
  setHttpIdleTimeoutMs(timeoutMs) {
    if (!Number.isFinite(timeoutMs) || timeoutMs < 0) {
      throw new Error(`Invalid httpIdleTimeoutMs setting: ${String(timeoutMs)}`);
    }
    this.globalSettings.httpIdleTimeoutMs = Math.floor(timeoutMs);
    this.markModified("httpIdleTimeoutMs");
    this.save();
  }
  /** Read from global settings only because warming costs money. */
  getCacheWarmingMode() {
    const mode = this.globalSettings.cacheWarming;
    return mode !== void 0 && CACHE_WARMING_MODES.includes(mode) ? mode : "streaming";
  }
  setCacheWarmingMode(mode) {
    this.globalSettings.cacheWarming = mode;
    this.markModified("cacheWarming");
    this.save();
  }
  getProviderRetrySettings() {
    return {
      timeoutMs: this.settings.retry?.provider?.timeoutMs,
      maxRetries: this.settings.retry?.provider?.maxRetries,
      maxRetryDelayMs: this.settings.retry?.provider?.maxRetryDelayMs ?? 6e4
    };
  }
  getWebSocketConnectTimeoutMs() {
    return parseTimeoutSetting(this.settings.websocketConnectTimeoutMs, "websocketConnectTimeoutMs");
  }
  getHideThinkingBlock() {
    return this.settings.hideThinkingBlock ?? false;
  }
  getShowCacheMissNotices() {
    return this.settings.showCacheMissNotices ?? false;
  }
  getExternalEditorCommand() {
    const configuredEditor = this.settings.externalEditor;
    if (typeof configuredEditor === "string" && configuredEditor.trim() !== "") {
      return configuredEditor;
    }
    const environmentEditor = process.env.VISUAL || process.env.EDITOR;
    if (environmentEditor) {
      return environmentEditor;
    }
    return process.platform === "win32" ? "notepad" : "nano";
  }
  setHideThinkingBlock(hide) {
    this.globalSettings.hideThinkingBlock = hide;
    this.markModified("hideThinkingBlock");
    this.save();
  }
  setShowCacheMissNotices(show) {
    this.globalSettings.showCacheMissNotices = show;
    this.markModified("showCacheMissNotices");
    this.save();
  }
  getShellPath() {
    const shellPath = this.settings.shellPath;
    return shellPath ? normalizePath(shellPath) : shellPath;
  }
  setShellPath(path4) {
    this.globalSettings.shellPath = path4;
    this.markModified("shellPath");
    this.save();
  }
  getQuietStartup() {
    const value = this.settings.quietStartup;
    return value === true || value === "header" ? value : false;
  }
  setQuietStartup(quiet) {
    this.globalSettings.quietStartup = quiet;
    this.markModified("quietStartup");
    this.save();
  }
  getDefaultProjectTrust() {
    const value = this.globalSettings.defaultProjectTrust;
    return value === "always" || value === "never" ? value : "ask";
  }
  setDefaultProjectTrust(defaultProjectTrust) {
    this.globalSettings.defaultProjectTrust = defaultProjectTrust;
    this.markModified("defaultProjectTrust");
    this.save();
  }
  getShellCommandPrefix() {
    return this.settings.shellCommandPrefix;
  }
  setShellCommandPrefix(prefix) {
    this.globalSettings.shellCommandPrefix = prefix;
    this.markModified("shellCommandPrefix");
    this.save();
  }
  getNpmCommand() {
    return this.settings.npmCommand ? [...this.settings.npmCommand] : void 0;
  }
  setNpmCommand(command) {
    this.globalSettings.npmCommand = command ? [...command] : void 0;
    this.markModified("npmCommand");
    this.save();
  }
  getCollapseChangelog() {
    return this.settings.collapseChangelog ?? false;
  }
  setCollapseChangelog(collapse) {
    this.globalSettings.collapseChangelog = collapse;
    this.markModified("collapseChangelog");
    this.save();
  }
  getEnableInstallTelemetry() {
    return this.settings.enableInstallTelemetry ?? true;
  }
  setEnableInstallTelemetry(enabled) {
    this.globalSettings.enableInstallTelemetry = enabled;
    this.markModified("enableInstallTelemetry");
    this.save();
  }
  getEnableAnalytics() {
    return this.settings.enableAnalytics ?? false;
  }
  getTrackingId() {
    return this.settings.trackingId;
  }
  /** Set the analytics opt-in preference; generates a tracking identifier on first opt-in */
  setEnableAnalytics(enabled) {
    this.globalSettings.enableAnalytics = enabled;
    this.markModified("enableAnalytics");
    if (enabled && !this.globalSettings.trackingId) {
      this.globalSettings.trackingId = randomUUID2();
      this.markModified("trackingId");
    }
    this.save();
  }
  /**
   * Stable ID of this installation, e.g. sent to OpenAI as its agent host ID.
   * Created on first use. Project settings are ignored so a committed project
   * settings file cannot give every clone the same ID.
   */
  getOrCreateDeviceId() {
    if (!this.globalSettings.deviceId) {
      this.globalSettings.deviceId = randomUUID2();
      this.markModified("deviceId");
      this.save();
    }
    return this.globalSettings.deviceId;
  }
  getPackages() {
    return [...this.settings.packages ?? []];
  }
  setPackages(packages) {
    this.globalSettings.packages = packages;
    this.markModified("packages");
    this.save();
  }
  setProjectPackages(packages) {
    this.updateProjectSettings("packages", (settings) => {
      settings.packages = packages;
    });
  }
  getExtensionPaths() {
    return [...this.settings.extensions ?? []];
  }
  setExtensionPaths(paths) {
    this.globalSettings.extensions = paths;
    this.markModified("extensions");
    this.save();
  }
  setProjectExtensionPaths(paths) {
    this.updateProjectSettings("extensions", (settings) => {
      settings.extensions = paths;
    });
  }
  getSkillPaths() {
    return [...this.settings.skills ?? []];
  }
  setSkillPaths(paths) {
    this.globalSettings.skills = paths;
    this.markModified("skills");
    this.save();
  }
  setProjectSkillPaths(paths) {
    this.updateProjectSettings("skills", (settings) => {
      settings.skills = paths;
    });
  }
  getPromptTemplatePaths() {
    return [...this.settings.prompts ?? []];
  }
  setPromptTemplatePaths(paths) {
    this.globalSettings.prompts = paths;
    this.markModified("prompts");
    this.save();
  }
  setProjectPromptTemplatePaths(paths) {
    this.updateProjectSettings("prompts", (settings) => {
      settings.prompts = paths;
    });
  }
  getThemePaths() {
    return [...this.settings.themes ?? []];
  }
  setThemePaths(paths) {
    this.globalSettings.themes = paths;
    this.markModified("themes");
    this.save();
  }
  setProjectThemePaths(paths) {
    this.updateProjectSettings("themes", (settings) => {
      settings.themes = paths;
    });
  }
  getEnableSkillCommands() {
    return this.settings.enableSkillCommands ?? true;
  }
  setEnableSkillCommands(enabled) {
    this.globalSettings.enableSkillCommands = enabled;
    this.markModified("enableSkillCommands");
    this.save();
  }
  getThinkingBudgets() {
    return this.settings.thinkingBudgets;
  }
  getTerminalCapabilityOverrides() {
    const terminal = this.settings.terminal;
    const images = terminal?.images;
    return {
      ...images === "kitty" || images === "iterm2" ? { images } : images === false ? { images: null } : {},
      ...typeof terminal?.trueColor === "boolean" ? { trueColor: terminal.trueColor } : {},
      ...typeof terminal?.hyperlinks === "boolean" ? { hyperlinks: terminal.hyperlinks } : {}
    };
  }
  getShowImages() {
    return this.settings.terminal?.showImages ?? true;
  }
  setShowImages(show) {
    if (!this.globalSettings.terminal) {
      this.globalSettings.terminal = {};
    }
    this.globalSettings.terminal.showImages = show;
    this.markModified("terminal", "showImages");
    this.save();
  }
  getImageWidthCells() {
    const width = this.settings.terminal?.imageWidthCells;
    if (typeof width !== "number" || !Number.isFinite(width)) {
      return 60;
    }
    return Math.max(1, Math.floor(width));
  }
  setImageWidthCells(width) {
    if (!this.globalSettings.terminal) {
      this.globalSettings.terminal = {};
    }
    this.globalSettings.terminal.imageWidthCells = Math.max(1, Math.floor(width));
    this.markModified("terminal", "imageWidthCells");
    this.save();
  }
  getClearOnShrink() {
    if (this.settings.terminal?.clearOnShrink !== void 0) {
      return this.settings.terminal.clearOnShrink;
    }
    return process.env.PI_CLEAR_ON_SHRINK === "1";
  }
  setClearOnShrink(enabled) {
    if (!this.globalSettings.terminal) {
      this.globalSettings.terminal = {};
    }
    this.globalSettings.terminal.clearOnShrink = enabled;
    this.markModified("terminal", "clearOnShrink");
    this.save();
  }
  getShowTerminalProgress() {
    return this.settings.terminal?.showTerminalProgress ?? false;
  }
  setShowTerminalProgress(enabled) {
    if (!this.globalSettings.terminal) {
      this.globalSettings.terminal = {};
    }
    this.globalSettings.terminal.showTerminalProgress = enabled;
    this.markModified("terminal", "showTerminalProgress");
    this.save();
  }
  getTuiMode() {
    return this.settings.tuiMode === "regular" ? "regular" : "fullscreen";
  }
  setTuiMode(mode) {
    this.globalSettings.tuiMode = mode;
    this.markModified("tuiMode");
    this.save();
  }
  getFullscreenExitOutput() {
    return this.settings.fullscreenExitOutput === "resume-hint" ? "resume-hint" : "transcript";
  }
  setFullscreenExitOutput(output) {
    this.globalSettings.fullscreenExitOutput = output;
    this.markModified("fullscreenExitOutput");
    this.save();
  }
  getFullscreenScrollbar() {
    const mode = this.settings.fullscreenScrollbar;
    return mode === "always" || mode === "hidden" ? mode : "auto";
  }
  setFullscreenScrollbar(mode) {
    this.globalSettings.fullscreenScrollbar = mode;
    this.markModified("fullscreenScrollbar");
    this.save();
  }
  getFullscreenCopyOnSelect() {
    return this.settings.fullscreenCopyOnSelect ?? true;
  }
  setFullscreenCopyOnSelect(enabled) {
    this.globalSettings.fullscreenCopyOnSelect = enabled;
    this.markModified("fullscreenCopyOnSelect");
    this.save();
  }
  getFullscreenWheelScrollLines() {
    const lines = this.settings.fullscreenWheelScrollLines;
    return typeof lines === "number" && Number.isFinite(lines) ? Math.max(1, Math.min(100, Math.floor(lines))) : "auto";
  }
  setFullscreenWheelScrollLines(lines) {
    this.globalSettings.fullscreenWheelScrollLines = lines === "auto" ? lines : Math.max(1, Math.min(100, Math.floor(lines)));
    this.markModified("fullscreenWheelScrollLines");
    this.save();
  }
  getImageAutoResize() {
    return this.settings.images?.autoResize ?? true;
  }
  setImageAutoResize(enabled) {
    if (!this.globalSettings.images) {
      this.globalSettings.images = {};
    }
    this.globalSettings.images.autoResize = enabled;
    this.markModified("images", "autoResize");
    this.save();
  }
  getBlockImages() {
    return this.settings.images?.blockImages ?? false;
  }
  setBlockImages(blocked) {
    if (!this.globalSettings.images) {
      this.globalSettings.images = {};
    }
    this.globalSettings.images.blockImages = blocked;
    this.markModified("images", "blockImages");
    this.save();
  }
  getEnabledModels() {
    return this.settings.enabledModels;
  }
  /** The resolved `defaultTools` selection, or undefined when no settings layer sets it. */
  getDefaultTools() {
    const tools = this.settings.defaultTools;
    if (tools === void 0)
      return void 0;
    return resolveDefaultTools(Array.isArray(tools) ? tools.filter((tool) => typeof tool === "string") : []);
  }
  setEnabledModels(patterns) {
    this.globalSettings.enabledModels = patterns;
    this.markModified("enabledModels");
    this.save();
  }
  getDoubleEscapeAction() {
    return this.settings.doubleEscapeAction ?? "tree";
  }
  setDoubleEscapeAction(action) {
    this.globalSettings.doubleEscapeAction = action;
    this.markModified("doubleEscapeAction");
    this.save();
  }
  getTreeFilterMode() {
    const mode = this.settings.treeFilterMode;
    const valid = ["default", "no-tools", "user-only", "labeled-only", "all"];
    return mode && valid.includes(mode) ? mode : "default";
  }
  setTreeFilterMode(mode) {
    this.globalSettings.treeFilterMode = mode;
    this.markModified("treeFilterMode");
    this.save();
  }
  getShowHardwareCursor() {
    return this.settings.showHardwareCursor ?? process.env.PI_HARDWARE_CURSOR === "1";
  }
  setShowHardwareCursor(enabled) {
    this.globalSettings.showHardwareCursor = enabled;
    this.markModified("showHardwareCursor");
    this.save();
  }
  getEditorPaddingX() {
    return this.settings.editorPaddingX ?? 0;
  }
  setEditorPaddingX(padding) {
    this.globalSettings.editorPaddingX = Math.max(0, Math.min(3, Math.floor(padding)));
    this.markModified("editorPaddingX");
    this.save();
  }
  getOutputPad() {
    return this.settings.outputPad === 0 ? 0 : 1;
  }
  setOutputPad(padding) {
    this.globalSettings.outputPad = padding;
    this.markModified("outputPad");
    this.save();
  }
  getAutocompleteMaxVisible() {
    return this.settings.autocompleteMaxVisible ?? 5;
  }
  setAutocompleteMaxVisible(maxVisible) {
    this.globalSettings.autocompleteMaxVisible = Math.max(3, Math.min(20, Math.floor(maxVisible)));
    this.markModified("autocompleteMaxVisible");
    this.save();
  }
  getCodeBlockIndent() {
    return this.settings.markdown?.codeBlockIndent ?? "  ";
  }
  getMermaidRenderingMode() {
    const mode = this.settings.markdown?.mermaid;
    return mode === "off" || mode === "final" ? mode : "streaming";
  }
  setMermaidRenderingMode(mode) {
    this.globalSettings.markdown ??= {};
    this.globalSettings.markdown.mermaid = mode;
    this.markModified("markdown", "mermaid");
    this.save();
  }
  getWarnings() {
    return { ...this.settings.warnings ?? {} };
  }
  setWarnings(warnings) {
    this.globalSettings.warnings = { ...warnings };
    this.markModified("warnings");
    this.save();
  }
};

// target/vendor-pi/dist/core/tools/tool-definition-wrapper.js
function wrapToolDefinition(definition, ctxFactory) {
  return {
    name: definition.name,
    label: definition.label,
    description: definition.description,
    parameters: definition.parameters,
    outputSchema: definition.outputSchema,
    constrainedSampling: definition.constrainedSampling,
    prepareArguments: definition.prepareArguments,
    executionMode: definition.executionMode,
    execute: (toolCallId, params, signal, onUpdate, ctx) => definition.execute(toolCallId, params, signal, onUpdate, ctx ?? ctxFactory?.(toolCallId, signal))
  };
}

// target/vendor-pi/dist/core/extensions/wrapper.js
function wrapRegisteredTool(registeredTool, runner) {
  return wrapToolDefinition(registeredTool.definition, (toolCallId, signal) => runner.createToolContext(toolCallId, signal));
}
function wrapRegisteredTools(registeredTools, runner) {
  return registeredTools.map((tool) => wrapRegisteredTool(tool, runner));
}

// target/vendor-pi/dist/utils/mime.js
import { open } from "node:fs/promises";
var IMAGE_TYPE_SNIFF_BYTES = 4100;
var PNG_SIGNATURE = [137, 80, 78, 71, 13, 10, 26, 10];
function detectSupportedImageMimeType(buffer) {
  if (startsWith(buffer, [255, 216, 255])) {
    return buffer[3] === 247 ? null : "image/jpeg";
  }
  if (startsWith(buffer, PNG_SIGNATURE)) {
    return isPng(buffer) && !isAnimatedPng(buffer) ? "image/png" : null;
  }
  if (startsWithAscii(buffer, 0, "GIF87a") || startsWithAscii(buffer, 0, "GIF89a")) {
    return "image/gif";
  }
  if (startsWithAscii(buffer, 0, "RIFF") && startsWithAscii(buffer, 8, "WEBP")) {
    return "image/webp";
  }
  if (startsWithAscii(buffer, 0, "BM") && isBmp(buffer)) {
    return "image/bmp";
  }
  return null;
}
async function detectSupportedImageMimeTypeFromFile(filePath) {
  const fileHandle = await open(filePath, "r");
  try {
    const buffer = Buffer.alloc(IMAGE_TYPE_SNIFF_BYTES);
    const { bytesRead } = await fileHandle.read(buffer, 0, IMAGE_TYPE_SNIFF_BYTES, 0);
    return detectSupportedImageMimeType(buffer.subarray(0, bytesRead));
  } finally {
    await fileHandle.close();
  }
}
function isPng(buffer) {
  return buffer.length >= 16 && readUint32BE(buffer, PNG_SIGNATURE.length) === 13 && startsWithAscii(buffer, 12, "IHDR");
}
function isAnimatedPng(buffer) {
  let offset = PNG_SIGNATURE.length;
  while (offset + 8 <= buffer.length) {
    const chunkLength = readUint32BE(buffer, offset);
    const chunkTypeOffset = offset + 4;
    if (startsWithAscii(buffer, chunkTypeOffset, "acTL"))
      return true;
    if (startsWithAscii(buffer, chunkTypeOffset, "IDAT"))
      return false;
    const nextOffset = offset + 8 + chunkLength + 4;
    if (nextOffset <= offset || nextOffset > buffer.length)
      return false;
    offset = nextOffset;
  }
  return false;
}
function isBmp(buffer) {
  if (buffer.length < 26)
    return false;
  const declaredFileSize = readUint32LE(buffer, 2);
  const pixelDataOffset = readUint32LE(buffer, 10);
  const dibHeaderSize = readUint32LE(buffer, 14);
  if (declaredFileSize !== 0 && declaredFileSize < 26)
    return false;
  if (pixelDataOffset < 14 + dibHeaderSize)
    return false;
  if (declaredFileSize !== 0 && pixelDataOffset >= declaredFileSize)
    return false;
  let colorPlanes;
  let bitsPerPixel;
  if (dibHeaderSize === 12) {
    colorPlanes = readUint16LE(buffer, 22);
    bitsPerPixel = readUint16LE(buffer, 24);
  } else if (dibHeaderSize >= 40 && dibHeaderSize <= 124) {
    if (buffer.length < 30)
      return false;
    colorPlanes = readUint16LE(buffer, 26);
    bitsPerPixel = readUint16LE(buffer, 28);
  } else {
    return false;
  }
  return colorPlanes === 1 && [1, 4, 8, 16, 24, 32].includes(bitsPerPixel);
}
function readUint16LE(buffer, offset) {
  return (buffer[offset] ?? 0) + ((buffer[offset + 1] ?? 0) << 8);
}
function readUint32BE(buffer, offset) {
  return (buffer[offset] ?? 0) * 16777216 + ((buffer[offset + 1] ?? 0) << 16) + ((buffer[offset + 2] ?? 0) << 8) + (buffer[offset + 3] ?? 0);
}
function readUint32LE(buffer, offset) {
  return (buffer[offset] ?? 0) + ((buffer[offset + 1] ?? 0) << 8) + ((buffer[offset + 2] ?? 0) << 16) + (buffer[offset + 3] ?? 0) * 16777216;
}
function startsWith(buffer, bytes) {
  if (buffer.length < bytes.length)
    return false;
  return bytes.every((byte, index) => buffer[index] === byte);
}
function startsWithAscii(buffer, offset, text) {
  if (buffer.length < offset + text.length)
    return false;
  for (let index = 0; index < text.length; index++) {
    if (buffer[offset + index] !== text.charCodeAt(index))
      return false;
  }
  return true;
}

// target/vendor-pi/dist/extensions/tool-search/tool.js
var TOOL_SEARCH_TOOL_NAME = "tool_search";
var DEFAULT_TOOL_SEARCH_LIMIT = 8;
var STOP_WORDS = /* @__PURE__ */ new Set([
  "a",
  "an",
  "and",
  "are",
  "as",
  "at",
  "be",
  "by",
  "for",
  "from",
  "in",
  "is",
  "it",
  "of",
  "on",
  "or",
  "that",
  "the",
  "this",
  "to",
  "with"
]);
function stem(term) {
  if (term.length > 4 && term.endsWith("ies"))
    return `${term.slice(0, -3)}y`;
  if (term.length > 4 && /(ches|shes|sses|xes|zes)$/.test(term))
    return term.slice(0, -2);
  if (term.length > 3 && term.endsWith("s") && !term.endsWith("ss"))
    return term.slice(0, -1);
  return term;
}
function tokenize2(text) {
  return text.replace(/([a-z0-9])([A-Z])/g, "$1 $2").replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2").toLowerCase().split(/[^a-z0-9]+/).filter((term) => term.length > 0 && !STOP_WORDS.has(term)).map(stem);
}
function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function schemaText(schema, parts) {
  if (!isObject(schema))
    return;
  if (typeof schema.description === "string")
    parts.push(schema.description);
  if (isObject(schema.properties)) {
    for (const [name, property] of Object.entries(schema.properties)) {
      parts.push(name);
      schemaText(property, parts);
    }
  }
  schemaText(schema.items, parts);
  for (const key of ["anyOf", "oneOf", "allOf"]) {
    const variants = schema[key];
    if (Array.isArray(variants))
      for (const variant of variants)
        schemaText(variant, parts);
  }
}
function createToolSearchDocument(tool, namespace) {
  const parts = [tool.name, tool.name.replaceAll("_", " "), tool.description];
  schemaText(tool.parameters, parts);
  if (namespace)
    parts.push(namespace.name, namespace.description ?? "", namespace.instructions ?? "");
  return { name: tool.name, text: parts.filter((part) => part.trim()).join(" ") };
}
var Bm25Ranker = class {
  k1;
  b;
  constructor(options = {}) {
    this.k1 = options.k1 ?? 1.2;
    this.b = options.b ?? 0.75;
  }
  rank(query, documents, limit) {
    const queryTerms = [...new Set(tokenize2(query))];
    if (queryTerms.length === 0 || documents.length === 0 || limit <= 0)
      return [];
    const termCounts = documents.map((document) => {
      const counts = /* @__PURE__ */ new Map();
      for (const term of tokenize2(document.text))
        counts.set(term, (counts.get(term) ?? 0) + 1);
      return counts;
    });
    const lengths = termCounts.map((counts) => [...counts.values()].reduce((sum, count) => sum + count, 0));
    const averageLength = lengths.reduce((sum, length) => sum + length, 0) / documents.length || 1;
    const idf = new Map(queryTerms.map((term) => {
      const frequency = termCounts.filter((counts) => counts.has(term)).length;
      return [term, Math.log(1 + (documents.length - frequency + 0.5) / (frequency + 0.5))];
    }));
    const matches = [];
    documents.forEach((document, index) => {
      let score = 0;
      for (const term of queryTerms) {
        const count = termCounts[index].get(term);
        if (!count)
          continue;
        const norm = this.k1 * (1 - this.b + this.b * lengths[index] / averageLength);
        score += (idf.get(term) ?? 0) * (count * (this.k1 + 1) / (count + norm));
      }
      if (score > 0)
        matches.push({ name: document.name, score });
    });
    return matches.sort((a, b) => b.score - a.score).slice(0, limit);
  }
};
var toolSearchSchema = typebox_exports.Object({
  query: typebox_exports.String({ description: "Search query for deferred tools." }),
  limit: typebox_exports.Optional(typebox_exports.Number({ description: `Maximum number of tools to return. Defaults to ${DEFAULT_TOOL_SEARCH_LIMIT}.` }))
});
function isSearchable(exposure) {
  return exposure === "codemode" || exposure === "deferred";
}
function searchAndLoad(tools, query, limit) {
  const active = tools.getActiveTools();
  const candidates = tools.getAllTools().filter((tool) => isSearchable(tool.exposure) && !active.includes(tool.name));
  const documents = candidates.map((tool) => createToolSearchDocument(tool, tool.namespace));
  const matches = new Bm25Ranker().rank(query, documents, limit);
  if (matches.length > 0)
    tools.setActiveTools([...active, ...matches.map((match2) => match2.name)]);
  return matches.map((match2) => ({
    name: match2.name,
    description: candidates.find((tool) => tool.name === match2.name)?.description ?? ""
  }));
}
var TOOL_SEARCH_DESCRIPTION = `# Tool discovery

Searches over deferred tool metadata with BM25 and exposes matching tools for the next model call.

Some of the tools, such as tools of MCP servers, may not have been provided to you upfront, and you should use this tool (\`${TOOL_SEARCH_TOOL_NAME}\`) to search for the required tools. For MCP tool discovery, always use \`${TOOL_SEARCH_TOOL_NAME}\`.`;
function createToolSearchToolDefinition(options = {}) {
  return {
    name: TOOL_SEARCH_TOOL_NAME,
    label: TOOL_SEARCH_TOOL_NAME,
    description: TOOL_SEARCH_DESCRIPTION,
    promptSnippet: "Search for tools that are not loaded yet and load the matches",
    parameters: toolSearchSchema,
    // Searching is not something scripts need; it changes what the model sees.
    exposure: "model-only",
    async execute(_toolCallId, { query, limit }) {
      if (query.trim() === "")
        throw new Error("query must not be empty");
      const max = limit ?? DEFAULT_TOOL_SEARCH_LIMIT;
      if (!Number.isInteger(max) || max <= 0)
        throw new Error("limit must be a positive integer");
      const tools = options.tools ? searchAndLoad(options.tools, query, max) : [];
      const text = tools.length === 0 ? "No matching tools found." : `Loaded ${tools.length} tool${tools.length === 1 ? "" : "s"}. They are available from your next call:
${tools.map((tool) => `- ${tool.name}: ${tool.description.trim().split(/\r?\n/)[0]}`).join("\n")}`;
      return { content: [{ type: "text", text }], details: { loaded: tools.map((tool) => tool.name) } };
    }
  };
}

// target/vendor-pi/dist/extensions/tool-search/index.js
function createToolSearchExtension() {
  return (pi) => {
    pi.registerTool({ ...createToolSearchToolDefinition({ tools: pi }), defaultActive: false });
  };
}
var tool_search_default = createToolSearchExtension();

// target/vendor-pi/dist/core/tools/bash.js
import { constants as constants3 } from "node:fs";
import { access as fsAccess } from "node:fs/promises";
import { constants as osConstants } from "node:os";
import { spawn as spawn2 } from "child_process";

// target/vendor-pi/dist/core/tools/output-accumulator.js
import { randomBytes } from "node:crypto";
import { createWriteStream } from "node:fs";
import { open as open2 } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join as join8 } from "node:path";

// target/vendor-pi/dist/core/tools/truncate.js
var DEFAULT_MAX_LINES = 2e3;
var DEFAULT_MAX_BYTES = 50 * 1024;
var GREP_MAX_LINE_LENGTH = 500;
function splitLinesForCounting(content) {
  if (content.length === 0) {
    return [];
  }
  const lines = content.split("\n");
  if (content.endsWith("\n")) {
    lines.pop();
  }
  return lines;
}
function formatSize(bytes) {
  if (bytes < 1024) {
    return `${bytes}B`;
  } else if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)}KB`;
  } else {
    return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  }
}
function truncateHead(content, options = {}) {
  const maxLines = options.maxLines ?? DEFAULT_MAX_LINES;
  const maxBytes = options.maxBytes ?? DEFAULT_MAX_BYTES;
  const totalBytes = Buffer.byteLength(content, "utf-8");
  const lines = splitLinesForCounting(content);
  const totalLines = lines.length;
  if (totalLines <= maxLines && totalBytes <= maxBytes) {
    return {
      content,
      truncated: false,
      truncatedBy: null,
      totalLines,
      totalBytes,
      outputLines: totalLines,
      outputBytes: totalBytes,
      lastLinePartial: false,
      firstLineExceedsLimit: false,
      maxLines,
      maxBytes
    };
  }
  const firstLineBytes = Buffer.byteLength(lines[0], "utf-8");
  if (firstLineBytes > maxBytes) {
    return {
      content: "",
      truncated: true,
      truncatedBy: "bytes",
      totalLines,
      totalBytes,
      outputLines: 0,
      outputBytes: 0,
      lastLinePartial: false,
      firstLineExceedsLimit: true,
      maxLines,
      maxBytes
    };
  }
  const outputLinesArr = [];
  let outputBytesCount = 0;
  let truncatedBy = "lines";
  for (let i = 0; i < lines.length && i < maxLines; i++) {
    const line = lines[i];
    const lineBytes = Buffer.byteLength(line, "utf-8") + (i > 0 ? 1 : 0);
    if (outputBytesCount + lineBytes > maxBytes) {
      truncatedBy = "bytes";
      break;
    }
    outputLinesArr.push(line);
    outputBytesCount += lineBytes;
  }
  if (outputLinesArr.length >= maxLines && outputBytesCount <= maxBytes) {
    truncatedBy = "lines";
  }
  const outputContent = outputLinesArr.join("\n");
  const finalOutputBytes = Buffer.byteLength(outputContent, "utf-8");
  return {
    content: outputContent,
    truncated: true,
    truncatedBy,
    totalLines,
    totalBytes,
    outputLines: outputLinesArr.length,
    outputBytes: finalOutputBytes,
    lastLinePartial: false,
    firstLineExceedsLimit: false,
    maxLines,
    maxBytes
  };
}
function truncateTail(content, options = {}) {
  const maxLines = options.maxLines ?? DEFAULT_MAX_LINES;
  const maxBytes = options.maxBytes ?? DEFAULT_MAX_BYTES;
  const totalBytes = Buffer.byteLength(content, "utf-8");
  const lines = splitLinesForCounting(content);
  const totalLines = lines.length;
  if (totalLines <= maxLines && totalBytes <= maxBytes) {
    return {
      content,
      truncated: false,
      truncatedBy: null,
      totalLines,
      totalBytes,
      outputLines: totalLines,
      outputBytes: totalBytes,
      lastLinePartial: false,
      firstLineExceedsLimit: false,
      maxLines,
      maxBytes
    };
  }
  const outputLinesArr = [];
  let outputBytesCount = 0;
  let truncatedBy = "lines";
  let lastLinePartial = false;
  for (let i = lines.length - 1; i >= 0 && outputLinesArr.length < maxLines; i--) {
    const line = lines[i];
    const lineBytes = Buffer.byteLength(line, "utf-8") + (outputLinesArr.length > 0 ? 1 : 0);
    if (outputBytesCount + lineBytes > maxBytes) {
      truncatedBy = "bytes";
      if (outputLinesArr.length === 0) {
        const truncatedLine = truncateStringToBytesFromEnd(line, maxBytes);
        outputLinesArr.unshift(truncatedLine);
        outputBytesCount = Buffer.byteLength(truncatedLine, "utf-8");
        lastLinePartial = true;
      }
      break;
    }
    outputLinesArr.unshift(line);
    outputBytesCount += lineBytes;
  }
  if (outputLinesArr.length >= maxLines && outputBytesCount <= maxBytes) {
    truncatedBy = "lines";
  }
  const outputContent = outputLinesArr.join("\n");
  const finalOutputBytes = Buffer.byteLength(outputContent, "utf-8");
  return {
    content: outputContent,
    truncated: true,
    truncatedBy,
    totalLines,
    totalBytes,
    outputLines: outputLinesArr.length,
    outputBytes: finalOutputBytes,
    lastLinePartial,
    firstLineExceedsLimit: false,
    maxLines,
    maxBytes
  };
}
function truncateStringToBytesFromEnd(str2, maxBytes) {
  const buf = Buffer.from(str2, "utf-8");
  if (buf.length <= maxBytes) {
    return str2;
  }
  let start = buf.length - maxBytes;
  while (start < buf.length && (buf[start] & 192) === 128) {
    start++;
  }
  return buf.slice(start).toString("utf-8");
}
function truncateLine(line, maxChars = GREP_MAX_LINE_LENGTH) {
  if (line.length <= maxChars) {
    return { text: line, wasTruncated: false };
  }
  return { text: `${line.slice(0, maxChars)}... [truncated]`, wasTruncated: true };
}

// target/vendor-pi/dist/core/tools/output-accumulator.js
function defaultTempFilePath(prefix) {
  const id = randomBytes(8).toString("hex");
  return join8(tmpdir(), `${prefix}-${id}.log`);
}
function byteLength(text) {
  return Buffer.byteLength(text, "utf-8");
}
var OutputAccumulator = class {
  maxLines;
  maxBytes;
  maxRollingBytes;
  tempFilePrefix;
  decoder = new TextDecoder();
  rawChunks = [];
  tailText = "";
  tailBytes = 0;
  tailStartsAtLineBoundary = true;
  totalRawBytes = 0;
  totalDecodedBytes = 0;
  completedLines = 0;
  totalLines = 0;
  currentLineBytes = 0;
  hasOpenLine = false;
  finished = false;
  tempFilePath;
  tempFileStream;
  constructor(options = {}) {
    this.maxLines = options.maxLines ?? DEFAULT_MAX_LINES;
    this.maxBytes = options.maxBytes ?? DEFAULT_MAX_BYTES;
    this.maxRollingBytes = Math.max(this.maxBytes * 2, 1);
    this.tempFilePrefix = options.tempFilePrefix ?? "pi-output";
  }
  append(data) {
    if (this.finished) {
      throw new Error("Cannot append to a finished output accumulator");
    }
    this.totalRawBytes += data.length;
    this.appendDecodedText(this.decoder.decode(data, { stream: true }));
    if (this.tempFileStream || this.shouldUseTempFile()) {
      this.ensureTempFile();
      this.tempFileStream?.write(data);
    } else if (data.length > 0) {
      this.rawChunks.push(data);
    }
  }
  finish() {
    if (this.finished) {
      return;
    }
    this.finished = true;
    this.appendDecodedText(this.decoder.decode());
    if (this.shouldUseTempFile()) {
      this.ensureTempFile();
    }
  }
  snapshot(options = {}) {
    const tailTruncation = truncateTail(this.getSnapshotText(), {
      maxLines: this.maxLines,
      maxBytes: this.maxBytes
    });
    const truncated = this.totalLines > this.maxLines || this.totalDecodedBytes > this.maxBytes;
    const truncatedBy = truncated ? tailTruncation.truncatedBy ?? (this.totalDecodedBytes > this.maxBytes ? "bytes" : "lines") : null;
    const truncation = {
      ...tailTruncation,
      truncated,
      truncatedBy,
      totalLines: this.totalLines,
      totalBytes: this.totalDecodedBytes,
      maxLines: this.maxLines,
      maxBytes: this.maxBytes
    };
    if (options.persistIfTruncated && truncation.truncated) {
      this.ensureTempFile();
    }
    return {
      content: truncation.content,
      truncation,
      fullOutputPath: this.tempFilePath
    };
  }
  async closeTempFile() {
    if (!this.tempFileStream) {
      return;
    }
    const stream = this.tempFileStream;
    this.tempFileStream = void 0;
    await new Promise((resolve5, reject) => {
      const onError = (error) => {
        stream.off("finish", onFinish);
        reject(error);
      };
      const onFinish = () => {
        stream.off("error", onError);
        resolve5();
      };
      stream.once("error", onError);
      stream.once("finish", onFinish);
      stream.end();
    });
  }
  /**
   * The complete output, for callers that can take more than the display snapshot. Call after
   * `finish()` and `closeTempFile()`. Output longer than `maxBytes` raw bytes keeps its first and
   * last `maxBytes / 2` bytes around an omission marker.
   */
  async readFullOutput(maxBytes) {
    if (!this.tempFilePath) {
      return { content: new TextDecoder().decode(Buffer.concat(this.rawChunks)), truncated: false };
    }
    const file = await open2(this.tempFilePath, "r");
    try {
      const size = (await file.stat()).size;
      if (size <= maxBytes) {
        return { content: new TextDecoder().decode(await file.readFile()), truncated: false };
      }
      const headBytes = Math.floor(maxBytes / 2);
      const tailBytes = maxBytes - headBytes;
      const head = Buffer.alloc(headBytes);
      const tail = Buffer.alloc(tailBytes);
      await file.read(head, 0, headBytes, 0);
      await file.read(tail, 0, tailBytes, size - tailBytes);
      const headText = new TextDecoder().decode(head, { stream: true });
      let tailStart = 0;
      while (tailStart < tail.length && (tail[tailStart] & 192) === 128)
        tailStart++;
      const tailText = new TextDecoder().decode(tail.subarray(tailStart));
      const omitted = size - headBytes - tailBytes;
      return { content: `${headText}

[... ${omitted} bytes omitted ...]

${tailText}`, truncated: true };
    } finally {
      await file.close();
    }
  }
  getLastLineBytes() {
    return this.currentLineBytes;
  }
  appendDecodedText(text) {
    if (text.length === 0) {
      return;
    }
    const bytes = byteLength(text);
    this.totalDecodedBytes += bytes;
    this.tailText += text;
    this.tailBytes += bytes;
    if (this.tailBytes > this.maxRollingBytes * 2) {
      this.trimTail();
    }
    let newlines = 0;
    let lastNewline = -1;
    for (let i = text.indexOf("\n"); i !== -1; i = text.indexOf("\n", i + 1)) {
      newlines++;
      lastNewline = i;
    }
    if (newlines === 0) {
      this.currentLineBytes += bytes;
      this.hasOpenLine = true;
    } else {
      this.completedLines += newlines;
      const tail = text.slice(lastNewline + 1);
      this.currentLineBytes = byteLength(tail);
      this.hasOpenLine = tail.length > 0;
    }
    this.totalLines = this.completedLines + (this.hasOpenLine ? 1 : 0);
  }
  trimTail() {
    const buffer = Buffer.from(this.tailText, "utf-8");
    if (buffer.length <= this.maxRollingBytes) {
      this.tailBytes = buffer.length;
      return;
    }
    let start = buffer.length - this.maxRollingBytes;
    while (start < buffer.length && (buffer[start] & 192) === 128) {
      start++;
    }
    this.tailStartsAtLineBoundary = start === 0 ? this.tailStartsAtLineBoundary : buffer[start - 1] === 10;
    this.tailText = buffer.subarray(start).toString("utf-8");
    this.tailBytes = byteLength(this.tailText);
  }
  getSnapshotText() {
    if (this.tailStartsAtLineBoundary) {
      return this.tailText;
    }
    const firstNewline = this.tailText.indexOf("\n");
    return firstNewline === -1 ? this.tailText : this.tailText.slice(firstNewline + 1);
  }
  shouldUseTempFile() {
    return this.totalRawBytes > this.maxBytes || this.totalDecodedBytes > this.maxBytes || this.totalLines > this.maxLines;
  }
  ensureTempFile() {
    if (this.tempFilePath) {
      return;
    }
    this.tempFilePath = defaultTempFilePath(this.tempFilePrefix);
    this.tempFileStream = createWriteStream(this.tempFilePath);
    for (const chunk of this.rawChunks) {
      this.tempFileStream.write(chunk);
    }
    this.rawChunks = [];
  }
};

// target/vendor-pi/dist/modes/interactive/theme/theme.js
var theme_exports = {};
__reExport(theme_exports, host_star3);
import * as host_star3 from "yapi:pi/host";

// target/vendor-pi/dist/modes/interactive/components/keybinding-hints.js
function formatKeyPart(part, options) {
  const displayPart = process.platform === "darwin" && part.toLowerCase() === "alt" ? "option" : part;
  return options.capitalize ? displayPart.charAt(0).toUpperCase() + displayPart.slice(1) : displayPart;
}
function formatKeyText(key, options = {}) {
  return key.split("/").map((k) => k.split("+").map((part) => formatKeyPart(part, options)).join("+")).join("/");
}
function formatKeys(keys, options = {}) {
  if (keys.length === 0)
    return "";
  return formatKeyText(keys.join("/"), options);
}
function keyText(keybinding) {
  return formatKeys(getKeybindings().getKeys(keybinding));
}
function keyDisplayText(keybinding) {
  return formatKeys(getKeybindings().getKeys(keybinding), { capitalize: true });
}
function keyHint(keybinding, description) {
  return theme_exports.theme.fg("dim", keyText(keybinding)) + theme_exports.theme.fg("muted", ` ${description}`);
}
function rawKeyHint(key, description) {
  return theme_exports.theme.fg("dim", formatKeyText(key)) + theme_exports.theme.fg("muted", ` ${description}`);
}

// target/vendor-pi/dist/modes/interactive/components/visual-truncate.js
function truncateToVisualLines(text, maxVisualLines, width, paddingX = 0, keep = "end") {
  if (!text) {
    return { visualLines: [], skippedCount: 0 };
  }
  const tempText = new Text(text, paddingX, 0);
  const allVisualLines = tempText.render(width);
  if (allVisualLines.length <= maxVisualLines) {
    return { visualLines: allVisualLines, skippedCount: 0 };
  }
  const truncatedLines = keep === "start" ? allVisualLines.slice(0, maxVisualLines) : allVisualLines.slice(-maxVisualLines);
  const skippedCount = allVisualLines.length - maxVisualLines;
  return { visualLines: truncatedLines, skippedCount };
}
var VisualLinePreview = class {
  options;
  cachedWidth;
  cachedLines;
  constructor(options) {
    this.options = options;
  }
  render(width) {
    if (this.cachedLines === void 0 || this.cachedWidth !== width) {
      const { text, maxVisualLines, keep, formatHint } = this.options;
      const preview = truncateToVisualLines(text, maxVisualLines, width, 0, keep);
      const lines = preview.visualLines;
      if (preview.skippedCount > 0) {
        const hint = truncateToWidth(formatHint(preview.skippedCount), width, "...");
        this.cachedLines = keep === "start" ? [...lines, hint] : [hint, ...lines];
      } else {
        this.cachedLines = lines;
      }
      this.cachedWidth = width;
    }
    return this.cachedLines;
  }
  invalidate() {
    this.cachedWidth = void 0;
    this.cachedLines = void 0;
  }
};

// target/vendor-pi/dist/core/tools/render-utils.js
import * as os from "node:os";
import { pathToFileURL } from "node:url";

// target/vendor-pi/dist/utils/ansi.js
function ansiRegex({ onlyFirst = false } = {}) {
  const ST = "(?:\\u0007|\\u001B\\u005C|\\u009C)";
  const osc = `(?:\\u001B\\][\\s\\S]*?${ST})`;
  const csi = "[\\u001B\\u009B][[\\]()#;?]*(?:\\d{1,4}(?:[;:]\\d{0,4})*)?[\\dA-PR-TZcf-nq-uy=><~]";
  const pattern = `${osc}|${csi}`;
  return new RegExp(pattern, onlyFirst ? void 0 : "g");
}
var regex = ansiRegex();
function stripAnsi(value) {
  if (typeof value !== "string") {
    throw new TypeError(`Expected a \`string\`, got \`${typeof value}\``);
  }
  if (!value.includes("\x1B") && !value.includes("\x9B")) {
    return value;
  }
  return value.replace(regex, "");
}

// target/vendor-pi/dist/core/tools/render-utils.js
function shortenPath(path4) {
  if (typeof path4 !== "string")
    return "";
  const home = os.homedir();
  if (path4.startsWith(home)) {
    return `~${path4.slice(home.length)}`;
  }
  return path4;
}
function linkPath(styledText, rawPath, cwd) {
  if (!getCapabilities().hyperlinks)
    return styledText;
  const absolutePath = resolvePath(rawPath, cwd);
  return hyperlink(styledText, pathToFileURL(absolutePath).href);
}
function str(value) {
  if (typeof value === "string")
    return value;
  if (value == null)
    return "";
  return null;
}
function replaceTabs(text) {
  return text.replace(/\t/g, "   ");
}
function normalizeDisplayText(text) {
  return text.replace(/\r/g, "");
}
function getTextOutput(result, showImages) {
  if (!result)
    return "";
  const textBlocks = result.content.filter((c) => c.type === "text");
  const imageBlocks = result.content.filter((c) => c.type === "image");
  let output = textBlocks.map((c) => sanitizeBinaryOutput(stripAnsi(c.text || "")).replace(/\r/g, "")).join("\n");
  const caps = getCapabilities();
  if (imageBlocks.length > 0 && (!caps.images || !showImages)) {
    const imageIndicators = imageBlocks.map((img) => {
      const mimeType = img.mimeType ?? "image/unknown";
      const dims = img.data && img.mimeType ? getImageDimensions(img.data, img.mimeType) ?? void 0 : void 0;
      return imageFallback(mimeType, dims);
    }).join("\n");
    output = output ? `${output}
${imageIndicators}` : imageIndicators;
  }
  return output;
}
var COLLAPSED_ARGS_CHARS = 100;
function formatToolCallWithArgs(title, args, theme31, expanded) {
  const header = theme31.fg("toolTitle", theme31.bold(title));
  if (args == null)
    return header;
  const entries = typeof args === "object" && !Array.isArray(args) ? Object.entries(args) : [["args", args]];
  if (entries.length === 0)
    return header;
  if (expanded) {
    const lines = entries.map(([key, value]) => {
      const text = typeof value === "string" ? value : JSON.stringify(value, null, 2) ?? String(value);
      return `  ${key}: ${replaceTabs(text).replace(/\r/g, "").split("\n").join("\n    ")}`;
    });
    return `${header}
${theme31.fg("muted", lines.join("\n"))}`;
  }
  const pairs = entries.map(([key, value]) => `${key}=${JSON.stringify(value) ?? String(value)}`).join(" ");
  const preview = pairs.length > COLLAPSED_ARGS_CHARS ? `${pairs.slice(0, COLLAPSED_ARGS_CHARS - 3)}...` : pairs;
  return `${header} ${theme31.fg("muted", preview)}`;
}
function invalidArgText(theme31) {
  return theme31.fg("error", "[invalid arg]");
}
function renderToolPath(rawPath, theme31, cwd, options) {
  if (rawPath === null)
    return invalidArgText(theme31);
  const value = rawPath || options?.emptyFallback;
  if (!value)
    return theme31.fg("toolOutput", "...");
  return linkPath(theme31.fg("accent", shortenPath(value)), value, cwd);
}

// target/vendor-pi/dist/core/tools/renderers/bash.js
var BASH_PREVIEW_LINES = 5;
var BASH_UPDATE_THROTTLE_MS = 100;
function formatDuration(ms) {
  const seconds = ms / 1e3;
  if (seconds < 60)
    return `${seconds.toFixed(1)}s`;
  const totalSeconds = Math.floor(seconds);
  const minutes = Math.floor(totalSeconds / 60);
  const remainder = totalSeconds % 60;
  if (minutes < 60)
    return `${minutes}m ${remainder}s`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m ${remainder}s`;
}
function formatShellCall(args, prompt) {
  const command = str(args?.command);
  const timeout = args?.timeout;
  const timeoutSuffix = timeout ? theme_exports.theme.fg("muted", ` (timeout ${timeout}s)`) : "";
  const commandDisplay = command === null ? invalidArgText(theme_exports.theme) : command ? command : theme_exports.theme.fg("toolOutput", "...");
  return theme_exports.theme.fg("toolTitle", theme_exports.theme.bold(`${prompt} ${commandDisplay}`)) + timeoutSuffix;
}
function rebuildBashResultRenderComponent(component, result, options, showImages, startedAt, endedAt) {
  component.clear();
  let output = getTextOutput(result, showImages).trim();
  const truncation = result.details?.truncation;
  const fullOutputPath = result.details?.fullOutputPath;
  if (!options.isPartial && truncation?.truncated && fullOutputPath && output.endsWith("]")) {
    const footerStart = output.lastIndexOf("\n\n[");
    if (footerStart !== -1 && output.slice(footerStart).includes(fullOutputPath)) {
      output = output.slice(0, footerStart).trimEnd();
    }
  }
  if (output) {
    const styledOutput = output.split("\n").map((line) => theme_exports.theme.fg("toolOutput", line)).join("\n");
    if (options.expanded) {
      component.addChild(new Text(`
${styledOutput}`, 0, 0));
    } else {
      component.addChild(new Spacer(1));
      component.addChild(new VisualLinePreview({
        text: styledOutput,
        maxVisualLines: BASH_PREVIEW_LINES,
        keep: "end",
        formatHint: (hidden) => theme_exports.theme.fg("muted", `... (${hidden} earlier lines,`) + ` ${keyHint("app.tools.expand", "to expand")}${theme_exports.theme.fg("muted", ")")}`
      }));
    }
  }
  if (truncation?.truncated || fullOutputPath) {
    const warnings = [];
    if (fullOutputPath) {
      warnings.push(`Full output: ${fullOutputPath}`);
    }
    if (truncation?.truncated) {
      if (truncation.truncatedBy === "lines") {
        warnings.push(`Truncated: showing ${truncation.outputLines} of ${truncation.totalLines} lines`);
      } else {
        warnings.push(`Truncated: ${truncation.outputLines} lines shown (${formatSize(truncation.maxBytes ?? DEFAULT_MAX_BYTES)} limit)`);
      }
    }
    component.addChild(new Text(`
${theme_exports.theme.fg("warning", `[${warnings.join(". ")}]`)}`, 0, 0));
  }
  if (startedAt !== void 0) {
    const label = options.isPartial ? "Elapsed" : "Took";
    const endTime = endedAt ?? Date.now();
    component.addChild(new Text(`
${theme_exports.theme.fg("muted", `${label} ${formatDuration(endTime - startedAt)}`)}`, 0, 0));
  }
}
function createShellRenderers(prompt) {
  return {
    renderCall(args, _theme, context) {
      const state = context.state;
      if (context.executionStarted && state.startedAt === void 0) {
        state.startedAt = Date.now();
        state.endedAt = void 0;
      }
      const text = context.lastComponent ?? new Text("", 0, 0);
      text.setText(formatShellCall(args, prompt));
      return text;
    },
    renderResult(result, options, _theme, context) {
      const state = context.state;
      if (state.startedAt !== void 0 && options.isPartial && !state.interval) {
        state.interval = setInterval(() => context.invalidate(), 1e3);
      }
      if (!options.isPartial || context.isError) {
        state.endedAt ??= Date.now();
        if (state.interval) {
          clearInterval(state.interval);
          state.interval = void 0;
        }
      }
      const component = context.lastComponent ?? new Container();
      rebuildBashResultRenderComponent(component, result, options, context.showImages, state.startedAt, state.endedAt);
      component.invalidate();
      return component;
    }
  };
}

// target/vendor-pi/dist/core/tools/bash.js
var MAX_TIMEOUT_MS = 2147483647;
var STRUCTURED_OUTPUT_MAX_BYTES = 1024 * 1024;
var MAX_TIMEOUT_SECONDS = MAX_TIMEOUT_MS / 1e3;
function resolveTimeoutMs(timeout) {
  if (timeout === void 0)
    return void 0;
  if (!Number.isFinite(timeout) || timeout <= 0) {
    throw new Error("Invalid timeout: must be a finite number of seconds");
  }
  const timeoutMs = timeout * 1e3;
  if (timeoutMs > MAX_TIMEOUT_MS) {
    throw new Error(`Invalid timeout: maximum is ${MAX_TIMEOUT_SECONDS} seconds`);
  }
  return timeoutMs;
}
var bashSchema = typebox_exports.Object({
  command: typebox_exports.String({ description: "Shell command to execute" }),
  timeout: typebox_exports.Optional(typebox_exports.Number({ description: "Timeout in seconds (optional, no default timeout)" }))
});
var bashToolSystemPromptContribution = {
  snippet: "Execute bash commands (ls, grep, find, etc.)",
  guidelines: ["You can inspect PI_* environment variables for current model and session details."]
};
var bashOutputSchema = typebox_exports.Object({
  output: typebox_exports.String({ description: "Combined stdout and stderr, possibly truncated" }),
  truncated: typebox_exports.Boolean(),
  full_output_path: typebox_exports.Optional(typebox_exports.String({ description: "Full output, when truncated" })),
  exit_code: typebox_exports.Number(),
  wall_time_seconds: typebox_exports.Number()
});
function createLocalShellOperations(shellName, resolveShellConfig) {
  return {
    exec: async (command, cwd, { onData, signal, timeout, env }) => {
      const timeoutMs = resolveTimeoutMs(timeout);
      if (signal?.aborted) {
        throw new Error("aborted");
      }
      const shellConfig = resolveShellConfig();
      try {
        await fsAccess(cwd, constants3.F_OK);
      } catch {
        throw new Error(`Working directory does not exist: ${cwd}
Cannot execute ${shellName} commands.`);
      }
      const commandFromStdin = shellConfig.commandTransport === "stdin";
      const child = spawn2(shellConfig.shell, commandFromStdin ? shellConfig.args : [...shellConfig.args, command], {
        cwd,
        detached: process.platform !== "win32",
        env: env ?? getShellEnv(),
        stdio: [commandFromStdin ? "pipe" : "ignore", "pipe", "pipe"],
        windowsHide: true
      });
      if (commandFromStdin) {
        child.stdin?.on("error", () => {
        });
        child.stdin?.end(command);
      }
      if (child.pid)
        trackDetachedChildPid(child.pid);
      let timedOut = false;
      let timeoutHandle;
      const onAbort = () => {
        if (child.pid)
          killProcessTree(child.pid);
      };
      try {
        if (timeoutMs !== void 0) {
          timeoutHandle = setTimeout(() => {
            timedOut = true;
            if (child.pid)
              killProcessTree(child.pid);
          }, timeoutMs);
        }
        child.stdout?.on("data", onData);
        child.stderr?.on("data", onData);
        if (signal) {
          if (signal.aborted)
            onAbort();
          else
            signal.addEventListener("abort", onAbort, { once: true });
        }
        const exitCode = await waitForChildProcess(child);
        if (signal?.aborted) {
          throw new Error("aborted");
        }
        if (timedOut) {
          throw new Error(`timeout:${timeout}`);
        }
        const signalCode = child.signalCode;
        return { exitCode: exitCode ?? (signalCode ? 128 + (osConstants.signals[signalCode] ?? 0) : 1) };
      } finally {
        if (child.pid)
          untrackDetachedChildPid(child.pid);
        if (timeoutHandle)
          clearTimeout(timeoutHandle);
        if (signal)
          signal.removeEventListener("abort", onAbort);
      }
    }
  };
}
function createLocalBashOperations(options) {
  return createLocalShellOperations("bash", () => getShellConfig(options?.shellPath));
}
function resolveSpawnContext(command, cwd, spawnHook, exposeSessionEnvironment, ctx) {
  const env = { ...getShellEnv() };
  delete env.PI_SESSION_ID;
  delete env.PI_SESSION_FILE;
  delete env.PI_PROVIDER;
  delete env.PI_MODEL;
  delete env.PI_REASONING_LEVEL;
  if (exposeSessionEnvironment && ctx) {
    const model = ctx.model;
    env.PI_SESSION_ID = ctx.sessionManager.getSessionId();
    const sessionFile = ctx.sessionManager.getSessionFile();
    if (sessionFile)
      env.PI_SESSION_FILE = sessionFile;
    if (model) {
      env.PI_PROVIDER = model.provider;
      env.PI_MODEL = model.id;
    }
    if (ctx.thinkingLevel)
      env.PI_REASONING_LEVEL = ctx.thinkingLevel;
  }
  const baseContext = { command, cwd, env };
  return spawnHook ? spawnHook(baseContext) : baseContext;
}
function createShellToolDefinition(cwd, config, options) {
  const ops = options?.operations ?? createLocalBashOperations({ shellPath: options?.shellPath });
  const commandPrefix = options?.commandPrefix;
  const exposeSessionEnvironment = options?.exposeSessionEnvironment ?? true;
  const spawnHook = options?.spawnHook;
  return {
    name: config.name,
    label: config.label,
    description: `Execute a ${config.shellName} command in the current working directory. Returns stdout and stderr. Output is truncated to last ${DEFAULT_MAX_LINES} lines or ${DEFAULT_MAX_BYTES / 1024}KB (whichever is hit first). If truncated, full output is saved to a temp file. Optionally provide a timeout in seconds.`,
    promptSnippet: config.promptSnippet,
    promptGuidelines: exposeSessionEnvironment && config.promptGuidelines ? [...config.promptGuidelines] : void 0,
    parameters: bashSchema,
    outputSchema: bashOutputSchema,
    constrainedSampling: { type: "json_schema", strict: "prefer" },
    async execute(_toolCallId, { command, timeout }, signal, onUpdate, ctx) {
      const resolvedCommand = commandPrefix ? `${commandPrefix}
${command}` : command;
      const spawnContext = resolveSpawnContext(resolvedCommand, ctx?.cwd || cwd, spawnHook, exposeSessionEnvironment, ctx);
      const output = new OutputAccumulator({ tempFilePrefix: config.tempFilePrefix });
      let acceptingOutput = true;
      let updateTimer;
      let updateDirty = false;
      let lastUpdateAt = 0;
      const emitOutputUpdate = () => {
        if (!onUpdate || !updateDirty)
          return;
        updateDirty = false;
        lastUpdateAt = Date.now();
        const snapshot = output.snapshot({ persistIfTruncated: true });
        onUpdate({
          content: [{ type: "text", text: snapshot.content || "" }],
          details: {
            truncation: snapshot.truncation.truncated ? snapshot.truncation : void 0,
            fullOutputPath: snapshot.fullOutputPath
          }
        });
      };
      const clearUpdateTimer = () => {
        if (updateTimer) {
          clearTimeout(updateTimer);
          updateTimer = void 0;
        }
      };
      const scheduleOutputUpdate = () => {
        if (!onUpdate)
          return;
        updateDirty = true;
        const delay = BASH_UPDATE_THROTTLE_MS - (Date.now() - lastUpdateAt);
        if (delay <= 0) {
          clearUpdateTimer();
          emitOutputUpdate();
          return;
        }
        updateTimer ??= setTimeout(() => {
          updateTimer = void 0;
          emitOutputUpdate();
        }, delay);
      };
      if (onUpdate) {
        onUpdate({ content: [], details: void 0 });
      }
      const handleData = (data) => {
        if (!acceptingOutput)
          return;
        output.append(data);
        scheduleOutputUpdate();
      };
      const finishOutput = async () => {
        acceptingOutput = false;
        output.finish();
        clearUpdateTimer();
        emitOutputUpdate();
        const snapshot = output.snapshot({ persistIfTruncated: true });
        await output.closeTempFile();
        return snapshot;
      };
      const formatOutput = (snapshot, emptyText = "(no output)") => {
        const truncation = snapshot.truncation;
        let text = snapshot.content || emptyText;
        let details;
        if (truncation.truncated) {
          details = { truncation, fullOutputPath: snapshot.fullOutputPath };
          const startLine = truncation.totalLines - truncation.outputLines + 1;
          const endLine = truncation.totalLines;
          if (truncation.lastLinePartial) {
            const lastLineSize = formatSize(output.getLastLineBytes());
            text += `

[Showing last ${formatSize(truncation.outputBytes)} of line ${endLine} (line is ${lastLineSize}). Full output: ${snapshot.fullOutputPath}]`;
          } else if (truncation.truncatedBy === "lines") {
            text += `

[Showing lines ${startLine}-${endLine} of ${truncation.totalLines}. Full output: ${snapshot.fullOutputPath}]`;
          } else {
            text += `

[Showing lines ${startLine}-${endLine} of ${truncation.totalLines} (${formatSize(DEFAULT_MAX_BYTES)} limit). Full output: ${snapshot.fullOutputPath}]`;
          }
        }
        return { text, details };
      };
      const appendStatus = (text, status) => `${text ? `${text}

` : ""}${status}`;
      const startedAt = performance.now();
      try {
        let exitCode;
        try {
          const result = await ops.exec(spawnContext.command, spawnContext.cwd, {
            onData: handleData,
            signal,
            timeout,
            env: spawnContext.env
          });
          exitCode = result.exitCode;
        } catch (err) {
          const snapshot2 = await finishOutput();
          const { text } = formatOutput(snapshot2, "");
          if (err instanceof Error && err.message === "aborted") {
            throw new Error(appendStatus(text, "Command aborted"));
          }
          if (err instanceof Error && err.message.startsWith("timeout:")) {
            const timeoutSecs = err.message.split(":")[1];
            throw new Error(appendStatus(text, `Command timed out after ${timeoutSecs} seconds`));
          }
          throw err;
        }
        const snapshot = await finishOutput();
        const { text: outputText, details } = formatOutput(snapshot);
        if (exitCode === null) {
          throw new Error(appendStatus(outputText, "Command terminated without an exit code"));
        }
        const wallTimeSeconds = Math.round((performance.now() - startedAt) / 100) / 10;
        const fullOutput = await output.readFullOutput(STRUCTURED_OUTPUT_MAX_BYTES);
        const structuredContent = {
          output: fullOutput.content,
          truncated: fullOutput.truncated,
          ...fullOutput.truncated && snapshot.fullOutputPath ? { full_output_path: snapshot.fullOutputPath } : {},
          exit_code: exitCode,
          wall_time_seconds: wallTimeSeconds
        };
        if (exitCode !== 0) {
          return {
            content: [{ type: "text", text: appendStatus(outputText, `Command exited with code ${exitCode}`) }],
            details,
            structuredContent,
            isError: true
          };
        }
        return { content: [{ type: "text", text: outputText }], details, structuredContent };
      } finally {
        clearUpdateTimer();
      }
    },
    ...createShellRenderers(config.prompt)
  };
}
var bashToolConfig = {
  name: "bash",
  label: "bash",
  shellName: "bash",
  prompt: "$",
  promptSnippet: bashToolSystemPromptContribution.snippet,
  promptGuidelines: bashToolSystemPromptContribution.guidelines,
  tempFilePrefix: "pi-bash"
};
function createBashToolDefinition(cwd, options) {
  return createShellToolDefinition(cwd, bashToolConfig, options);
}
function createBashTool(cwd, options) {
  const definition = createBashToolDefinition(cwd, options);
  const tool = wrapToolDefinition(definition);
  Object.assign(tool, {
    promptSnippet: definition.promptSnippet,
    promptGuidelines: definition.promptGuidelines
  });
  return tool;
}

// target/vendor-pi/dist/core/tools/edit.js
import { constants as constants4 } from "fs";
import { access as fsAccess2, readFile as fsReadFile, writeFile as fsWriteFile } from "fs/promises";

// target/vendor-pi/dist/core/tools/file-mutation-queue.js
import { realpath } from "node:fs/promises";
import { resolve as resolve3 } from "node:path";
var fileMutationQueues = /* @__PURE__ */ new Map();
var registrationQueue = Promise.resolve();
function isMissingPathError(error) {
  return typeof error === "object" && error !== null && "code" in error && (error.code === "ENOENT" || error.code === "ENOTDIR");
}
async function getMutationQueueKey(filePath) {
  const resolvedPath = resolve3(filePath);
  try {
    return await realpath(resolvedPath);
  } catch (error) {
    if (isMissingPathError(error)) {
      return resolvedPath;
    }
    throw error;
  }
}
async function withFileMutationQueue(filePath, fn) {
  const registration = registrationQueue.then(async () => {
    const key2 = await getMutationQueueKey(filePath);
    const currentQueue2 = fileMutationQueues.get(key2) ?? Promise.resolve();
    let releaseNext2;
    const nextQueue = new Promise((resolveQueue) => {
      releaseNext2 = resolveQueue;
    });
    const chainedQueue2 = currentQueue2.then(() => nextQueue);
    fileMutationQueues.set(key2, chainedQueue2);
    return { key: key2, currentQueue: currentQueue2, chainedQueue: chainedQueue2, releaseNext: releaseNext2 };
  });
  registrationQueue = registration.then(() => void 0, () => void 0);
  const { key, currentQueue, chainedQueue, releaseNext } = await registration;
  await currentQueue;
  try {
    return await fn();
  } finally {
    releaseNext();
    if (fileMutationQueues.get(key) === chainedQueue) {
      fileMutationQueues.delete(key);
    }
  }
}

// target/vendor-pi/dist/modes/interactive/components/diff.js
function parseDiffLine(line) {
  const match2 = line.match(/^([+-\s])(\s*\d*)\s(.*)$/);
  if (!match2)
    return null;
  return { prefix: match2[1], lineNum: match2[2], content: match2[3] };
}
function replaceTabs2(text) {
  return text.replace(/\t/g, "   ");
}
function renderIntraLineDiff(oldContent, newContent) {
  const wordDiff2 = diffWords(oldContent, newContent);
  let removedLine = "";
  let addedLine = "";
  let isFirstRemoved = true;
  let isFirstAdded = true;
  for (const part of wordDiff2) {
    if (part.removed) {
      let value = part.value;
      if (isFirstRemoved) {
        const leadingWs2 = value.match(/^(\s*)/)?.[1] || "";
        value = value.slice(leadingWs2.length);
        removedLine += leadingWs2;
        isFirstRemoved = false;
      }
      if (value) {
        removedLine += theme_exports.theme.inverse(value);
      }
    } else if (part.added) {
      let value = part.value;
      if (isFirstAdded) {
        const leadingWs2 = value.match(/^(\s*)/)?.[1] || "";
        value = value.slice(leadingWs2.length);
        addedLine += leadingWs2;
        isFirstAdded = false;
      }
      if (value) {
        addedLine += theme_exports.theme.inverse(value);
      }
    } else {
      removedLine += part.value;
      addedLine += part.value;
    }
  }
  return { removedLine, addedLine };
}
function renderDiff(diffText, _options = {}) {
  const lines = diffText.split("\n");
  const result = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    const parsed = parseDiffLine(line);
    if (!parsed) {
      result.push(theme_exports.theme.fg("toolDiffContext", line));
      i++;
      continue;
    }
    if (parsed.prefix === "-") {
      const removedLines = [];
      while (i < lines.length) {
        const p = parseDiffLine(lines[i]);
        if (!p || p.prefix !== "-")
          break;
        removedLines.push({ lineNum: p.lineNum, content: p.content });
        i++;
      }
      const addedLines = [];
      while (i < lines.length) {
        const p = parseDiffLine(lines[i]);
        if (!p || p.prefix !== "+")
          break;
        addedLines.push({ lineNum: p.lineNum, content: p.content });
        i++;
      }
      if (removedLines.length === 1 && addedLines.length === 1) {
        const removed = removedLines[0];
        const added = addedLines[0];
        const { removedLine, addedLine } = renderIntraLineDiff(replaceTabs2(removed.content), replaceTabs2(added.content));
        result.push(theme_exports.theme.fg("toolDiffRemoved", `-${removed.lineNum} ${removedLine}`));
        result.push(theme_exports.theme.fg("toolDiffAdded", `+${added.lineNum} ${addedLine}`));
      } else {
        for (const removed of removedLines) {
          result.push(theme_exports.theme.fg("toolDiffRemoved", `-${removed.lineNum} ${replaceTabs2(removed.content)}`));
        }
        for (const added of addedLines) {
          result.push(theme_exports.theme.fg("toolDiffAdded", `+${added.lineNum} ${replaceTabs2(added.content)}`));
        }
      }
    } else if (parsed.prefix === "+") {
      result.push(theme_exports.theme.fg("toolDiffAdded", `+${parsed.lineNum} ${replaceTabs2(parsed.content)}`));
      i++;
    } else {
      result.push(theme_exports.theme.fg("toolDiffContext", ` ${parsed.lineNum} ${replaceTabs2(parsed.content)}`));
      i++;
    }
  }
  return result.join("\n");
}

// target/vendor-pi/dist/core/tools/renderers/edit.js
function createEditCallRenderComponent() {
  return Object.assign(new Box(1, 1, (text) => text), {
    preview: void 0,
    previewArgsKey: void 0,
    previewPending: false,
    settledError: false
  });
}
function getEditCallRenderComponent(state, lastComponent) {
  if (lastComponent instanceof Box) {
    const component2 = lastComponent;
    state.callComponent = component2;
    return component2;
  }
  if (state.callComponent) {
    return state.callComponent;
  }
  const component = createEditCallRenderComponent();
  state.callComponent = component;
  return component;
}
function getRenderablePreviewInput(args) {
  if (!args) {
    return null;
  }
  const path4 = typeof args.path === "string" ? args.path : typeof args.file_path === "string" ? args.file_path : null;
  if (!path4) {
    return null;
  }
  if (Array.isArray(args.edits) && args.edits.length > 0 && args.edits.every((edit) => typeof edit?.oldText === "string" && typeof edit?.newText === "string")) {
    return { path: path4, edits: args.edits };
  }
  if (typeof args.oldText === "string" && typeof args.newText === "string") {
    return { path: path4, edits: [{ oldText: args.oldText, newText: args.newText }] };
  }
  return null;
}
function formatEditCall(args, theme31, cwd) {
  const pathDisplay = renderToolPath(str(args?.file_path ?? args?.path), theme31, cwd);
  return `${theme31.fg("toolTitle", theme31.bold("edit"))} ${pathDisplay}`;
}
function formatEditResult(args, preview, result, theme31, isError) {
  const rawPath = str(args?.file_path ?? args?.path);
  const previewDiff = preview && !("error" in preview) ? preview.diff : void 0;
  const previewError = preview && "error" in preview ? preview.error : void 0;
  if (isError) {
    const errorText = result.content.filter((c) => c.type === "text").map((c) => c.text || "").join("\n");
    if (!errorText || errorText === previewError) {
      return void 0;
    }
    return theme31.fg("error", errorText);
  }
  const resultDiff = result.details?.diff;
  if (resultDiff && resultDiff !== previewDiff) {
    return renderDiff(resultDiff, { filePath: rawPath ?? void 0 });
  }
  return void 0;
}
function getEditHeaderBg(preview, settledError, theme31) {
  if (preview) {
    if ("error" in preview) {
      return (text) => theme31.bg("toolErrorBg", text);
    }
    return (text) => theme31.bg("toolSuccessBg", text);
  }
  if (settledError) {
    return (text) => theme31.bg("toolErrorBg", text);
  }
  return (text) => theme31.bg("toolPendingBg", text);
}
function buildEditCallComponent(component, args, theme31, cwd) {
  component.setBgFn(getEditHeaderBg(component.preview, component.settledError, theme31));
  component.clear();
  component.addChild(new Text(formatEditCall(args, theme31, cwd), 0, 0));
  if (!component.preview) {
    return component;
  }
  const body = "error" in component.preview ? theme31.fg("error", component.preview.error) : renderDiff(component.preview.diff);
  component.addChild(new Spacer(1));
  component.addChild(new Text(body, 0, 0));
  return component;
}
function setEditPreview(component, preview, argsKey) {
  const current = component.preview;
  const changed = current === void 0 || ("error" in current && "error" in preview ? current.error !== preview.error : "error" in current !== "error" in preview) || !("error" in current) && !("error" in preview) && (current.diff !== preview.diff || current.firstChangedLine !== preview.firstChangedLine);
  component.preview = preview;
  component.previewArgsKey = argsKey;
  component.previewPending = false;
  return changed;
}
var editRenderers = {
  renderCall(args, theme31, context) {
    const component = getEditCallRenderComponent(context.state, context.lastComponent);
    const previewInput = getRenderablePreviewInput(args);
    const argsKey = previewInput ? JSON.stringify({ path: previewInput.path, edits: previewInput.edits }) : void 0;
    if (component.previewArgsKey !== argsKey) {
      component.preview = void 0;
      component.previewArgsKey = argsKey;
      component.previewPending = false;
      component.settledError = false;
    }
    if (context.argsComplete && previewInput && !component.preview && !component.previewPending) {
      component.previewPending = true;
      const requestKey = argsKey;
      void computeEditsDiff(previewInput.path, previewInput.edits, context.cwd).then((preview) => {
        if (component.previewArgsKey === requestKey) {
          setEditPreview(component, preview, requestKey);
          context.invalidate();
        }
      });
    }
    return buildEditCallComponent(component, args, theme31, context.cwd);
  },
  renderResult(result, _options, theme31, context) {
    const callComponent = context.state.callComponent;
    const previewInput = getRenderablePreviewInput(context.args);
    const argsKey = previewInput ? JSON.stringify({ path: previewInput.path, edits: previewInput.edits }) : void 0;
    const typedResult = result;
    const resultDiff = !context.isError ? typedResult.details?.diff : void 0;
    let changed = false;
    if (callComponent) {
      if (typeof resultDiff === "string") {
        changed = setEditPreview(callComponent, { diff: resultDiff, firstChangedLine: typedResult.details?.firstChangedLine }, argsKey) || changed;
      }
      if (callComponent.settledError !== context.isError) {
        callComponent.settledError = context.isError;
        changed = true;
      }
      if (changed) {
        buildEditCallComponent(callComponent, context.args, theme31, context.cwd);
      }
    }
    const output = formatEditResult(context.args, callComponent?.preview, typedResult, theme31, context.isError);
    const component = context.lastComponent ?? new Container();
    component.clear();
    if (!output) {
      return component;
    }
    component.addChild(new Spacer(1));
    component.addChild(new Text(output, 1, 0));
    return component;
  }
};

// target/vendor-pi/dist/core/tools/edit.js
var replaceEditSchema = typebox_exports.Object({
  oldText: typebox_exports.String({
    description: "Exact text for one targeted replacement. It must be unique in the original file and must not overlap with any other edits[].oldText in the same call."
  }),
  newText: typebox_exports.String({ description: "Replacement text for this targeted edit." })
}, {});
var editSchema = typebox_exports.Object({
  path: typebox_exports.String({ description: "Path to the file to edit (relative or absolute)" }),
  edits: typebox_exports.Array(replaceEditSchema, {
    description: "One or more targeted replacements. Each edit is matched against the original file, not incrementally. Do not include overlapping or nested edits. If two changes touch the same block or nearby lines, merge them into one edit instead."
  })
}, {});
var editToolSystemPromptContribution = {
  snippet: "Make precise file edits with exact text replacement, including multiple disjoint edits in one call",
  guidelines: [
    "Use edit for precise changes (edits[].oldText must match exactly)",
    "When changing multiple separate locations in one file, use one edit call with multiple entries in edits[] instead of multiple edit calls",
    "Each edits[].oldText is matched against the original file, not after earlier edits are applied. Do not emit overlapping or nested edits. Merge nearby changes into one edit.",
    "Keep edits[].oldText as small as possible while still being unique in the file. Do not pad with large unchanged regions."
  ]
};
function isSingleEditInput(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  const edit = value;
  return typeof edit.oldText === "string" && typeof edit.newText === "string";
}
var defaultEditOperations = {
  readFile: (path4) => fsReadFile(path4),
  writeFile: (path4, content) => fsWriteFile(path4, content, "utf-8"),
  access: (path4) => fsAccess2(path4, constants4.R_OK | constants4.W_OK)
};
function prepareEditArguments(input) {
  if (!input || typeof input !== "object") {
    return input;
  }
  const args = input;
  if (typeof args.edits === "string") {
    try {
      const parsed = JSON.parse(args.edits);
      if (Array.isArray(parsed)) {
        args.edits = parsed;
      } else if (isSingleEditInput(parsed)) {
        args.edits = [parsed];
      }
    } catch {
    }
  } else if (isSingleEditInput(args.edits)) {
    args.edits = [args.edits];
  }
  const legacy = args;
  if (typeof legacy.oldText !== "string" || typeof legacy.newText !== "string") {
    return args;
  }
  const edits = Array.isArray(legacy.edits) ? [...legacy.edits] : [];
  edits.push({ oldText: legacy.oldText, newText: legacy.newText });
  const { oldText: _oldText, newText: _newText, ...rest } = legacy;
  return { ...rest, edits };
}
function validateEditInput(input) {
  if (!Array.isArray(input.edits) || input.edits.length === 0) {
    throw new Error("Edit tool input is invalid. edits must contain at least one replacement.");
  }
  return { path: input.path, edits: input.edits };
}
function createEditToolDefinition(cwd, options) {
  const ops = options?.operations ?? defaultEditOperations;
  return {
    name: "edit",
    label: "edit",
    description: "Edit a single file using exact text replacement. Every edits[].oldText must match a unique, non-overlapping region of the original file. If two changes affect the same block or nearby lines, merge them into one edit instead of emitting overlapping edits. Do not include large unchanged regions just to connect distant changes.",
    promptSnippet: editToolSystemPromptContribution.snippet,
    promptGuidelines: [...editToolSystemPromptContribution.guidelines],
    parameters: editSchema,
    constrainedSampling: { type: "json_schema", strict: "prefer" },
    renderShell: "self",
    prepareArguments: prepareEditArguments,
    async execute(_toolCallId, input, signal, _onUpdate, ctx) {
      const { path: path4, edits } = validateEditInput(input);
      const absolutePath = resolveToCwd(path4, ctx?.cwd || cwd);
      return withFileMutationQueue(absolutePath, async () => {
        const throwIfAborted = () => {
          if (signal?.aborted)
            throw new Error("Operation aborted");
        };
        throwIfAborted();
        try {
          await ops.access(absolutePath);
        } catch (error) {
          throwIfAborted();
          const errorMessage = error instanceof Error && "code" in error ? `Error code: ${error.code}` : String(error);
          throw new Error(`Could not edit file: ${path4}. ${errorMessage}.`);
        }
        throwIfAborted();
        const buffer = await ops.readFile(absolutePath);
        const rawContent = buffer.toString("utf-8");
        throwIfAborted();
        const { bom, text: content } = splitBom(rawContent);
        const originalEnding = detectLineEnding(content);
        const normalizedContent = normalizeToLF(content);
        const { baseContent, newContent } = applyEditsToNormalizedContent(normalizedContent, edits, path4);
        throwIfAborted();
        const finalContent = bom + restoreLineEndings(newContent, originalEnding);
        await ops.writeFile(absolutePath, finalContent);
        throwIfAborted();
        const diffResult = generateDiffString(baseContent, newContent);
        const patch = generateUnifiedPatch(path4, baseContent, newContent);
        return {
          content: [
            {
              type: "text",
              text: `Successfully replaced ${edits.length} block(s) in ${path4}.`
            }
          ],
          details: { diff: diffResult.diff, patch, firstChangedLine: diffResult.firstChangedLine }
        };
      });
    },
    ...editRenderers
  };
}
function createEditTool(cwd, options) {
  return wrapToolDefinition(createEditToolDefinition(cwd, options));
}

// target/vendor-pi/dist/core/tools/find.js
import { createInterface as createInterface2 } from "node:readline";
import { spawn as spawn3 } from "child_process";
import path2 from "path";

// target/vendor-pi/dist/utils/tools-manager.js
import { spawnSync as spawnSync3 } from "child_process";
import { chmodSync, createWriteStream as createWriteStream2, existsSync as existsSync7, mkdirSync as mkdirSync5, readdirSync as readdirSync3, renameSync, rmSync } from "fs";
import { arch, platform } from "os";
import { join as join9 } from "path";
import { Readable } from "stream";
import { pipeline } from "stream/promises";

// target/vendor-pi/dist/utils/management-http.js
var RETRYABLE_STATUS_CODES = /* @__PURE__ */ new Set([408, 425, 429, 500, 502, 503, 504]);
async function fetchWithRetry(input, init = void 0, options = {}) {
  const maxRetries = options.maxRetries === void 0 || !Number.isFinite(options.maxRetries) ? 2 : Math.max(0, Math.floor(options.maxRetries));
  const retryOnStatus = options.retryOnStatus ?? true;
  const parentSignal = init?.signal ?? void 0;
  const timeoutSignal = options.timeoutMs !== void 0 && options.timeoutMs > 0 ? AbortSignal.timeout(options.timeoutMs) : void 0;
  const attemptTimeoutMs = options.attemptTimeoutMs !== void 0 && options.attemptTimeoutMs > 0 ? options.attemptTimeoutMs : void 0;
  for (let attempt = 0; ; attempt++) {
    parentSignal?.throwIfAborted();
    timeoutSignal?.throwIfAborted();
    const attemptTimeoutSignal = attemptTimeoutMs ? AbortSignal.timeout(attemptTimeoutMs) : void 0;
    const signals = [parentSignal, timeoutSignal, attemptTimeoutSignal].filter((signal2) => signal2 !== void 0);
    const signal = signals.length > 1 ? AbortSignal.any(signals) : signals[0];
    try {
      const response = await fetch(input, signal ? { ...init, signal } : init);
      const shouldRetry = retryOnStatus && RETRYABLE_STATUS_CODES.has(response.status) && attempt < maxRetries;
      if (!shouldRetry)
        return response;
      try {
        await response.body?.cancel();
      } catch {
      }
    } catch (error) {
      const attemptTimedOut = attemptTimeoutSignal?.aborted === true && !parentSignal?.aborted && !timeoutSignal?.aborted;
      if (parentSignal?.aborted || timeoutSignal?.aborted || error instanceof Error && error.name === "AbortError" && !attemptTimedOut && timeoutSignal === void 0 || attempt >= maxRetries) {
        throw error;
      }
    }
  }
}

// target/vendor-pi/dist/utils/tools-manager.js
var TOOLS_DIR = (0, config_exports.getBinDir)();
var NETWORK_TIMEOUT_MS = 1e4;
var DOWNLOAD_TIMEOUT_MS = 12e4;
function isOfflineModeEnabled() {
  const value = process.env.PI_OFFLINE;
  if (!value)
    return false;
  return value === "1" || value.toLowerCase() === "true" || value.toLowerCase() === "yes";
}
var TOOLS = {
  fd: {
    name: "fd",
    repo: "sharkdp/fd",
    binaryName: "fd",
    systemBinaryNames: ["fd", "fdfind"],
    tagPrefix: "v",
    getAssetName: (version, plat, architecture) => {
      if (plat === "darwin") {
        const archStr = architecture === "arm64" ? "aarch64" : "x86_64";
        return `fd-v${version}-${archStr}-apple-darwin.tar.gz`;
      } else if (plat === "linux") {
        const archStr = architecture === "arm64" ? "aarch64" : "x86_64";
        return `fd-v${version}-${archStr}-unknown-linux-musl.tar.gz`;
      } else if (plat === "win32") {
        const archStr = architecture === "arm64" ? "aarch64" : "x86_64";
        return `fd-v${version}-${archStr}-pc-windows-msvc.zip`;
      }
      return null;
    }
  },
  rg: {
    name: "ripgrep",
    repo: "BurntSushi/ripgrep",
    binaryName: "rg",
    tagPrefix: "",
    getAssetName: (version, plat, architecture) => {
      if (plat === "darwin") {
        const archStr = architecture === "arm64" ? "aarch64" : "x86_64";
        return `ripgrep-${version}-${archStr}-apple-darwin.tar.gz`;
      } else if (plat === "linux") {
        const archStr = architecture === "arm64" ? "aarch64" : "x86_64";
        return `ripgrep-${version}-${archStr}-unknown-linux-musl.tar.gz`;
      } else if (plat === "win32") {
        const archStr = architecture === "arm64" ? "aarch64" : "x86_64";
        return `ripgrep-${version}-${archStr}-pc-windows-msvc.zip`;
      }
      return null;
    }
  }
};
function commandExists(cmd) {
  try {
    const result = spawnSync3(cmd, ["--version"], { stdio: "pipe" });
    return result.error === void 0 || result.error === null;
  } catch {
    return false;
  }
}
function getToolPath(tool) {
  const config = TOOLS[tool];
  if (!config)
    return null;
  const localPath = join9(TOOLS_DIR, config.binaryName + (platform() === "win32" ? ".exe" : ""));
  if (existsSync7(localPath)) {
    return localPath;
  }
  const systemBinaryNames = config.systemBinaryNames ?? [config.binaryName];
  for (const systemBinaryName of systemBinaryNames) {
    if (commandExists(systemBinaryName)) {
      return systemBinaryName;
    }
  }
  return null;
}
async function getLatestVersion(repo) {
  const response = await fetchWithRetry(`https://github.com/${repo}/releases/latest`, {
    headers: { "User-Agent": `${config_exports.APP_NAME}-coding-agent` },
    redirect: "manual"
  }, { timeoutMs: NETWORK_TIMEOUT_MS });
  try {
    await response.body?.cancel();
  } catch {
  }
  const location = response.status >= 300 && response.status < 400 ? response.headers.get("location") : null;
  if (!location) {
    throw new Error(`Failed to resolve latest ${repo} release: HTTP ${response.status} without redirect`);
  }
  const tag = new URL(location, "https://github.com").pathname.split("/").pop();
  if (!tag || !location.includes("/releases/tag/")) {
    throw new Error(`Failed to resolve latest ${repo} release: unexpected redirect to ${location}`);
  }
  return decodeURIComponent(tag).replace(/^v/, "");
}
async function downloadFile(url, dest) {
  const response = await fetchWithRetry(url, void 0, { timeoutMs: DOWNLOAD_TIMEOUT_MS });
  if (!response.ok) {
    throw new Error(`Download failed with HTTP ${response.status}: ${url}`);
  }
  if (!response.body) {
    throw new Error("No response body");
  }
  const fileStream = createWriteStream2(dest);
  await pipeline(Readable.fromWeb(response.body), fileStream);
}
function findBinaryRecursively(rootDir, binaryFileName) {
  const stack = [rootDir];
  while (stack.length > 0) {
    const currentDir = stack.pop();
    if (!currentDir)
      continue;
    const entries = readdirSync3(currentDir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = join9(currentDir, entry.name);
      if (entry.isFile() && entry.name === binaryFileName) {
        return fullPath;
      }
      if (entry.isDirectory()) {
        stack.push(fullPath);
      }
    }
  }
  return null;
}
function formatSpawnFailure(result) {
  if (result.error?.message) {
    return result.error.message;
  }
  const stderr = result.stderr?.toString().trim();
  if (stderr) {
    return stderr;
  }
  const stdout = result.stdout?.toString().trim();
  if (stdout) {
    return stdout;
  }
  return `exit status ${result.status ?? "unknown"}`;
}
function runExtractionCommand(command, args) {
  const result = spawnSync3(command, args, { stdio: "pipe" });
  if (!result.error && result.status === 0) {
    return null;
  }
  return `${command}: ${formatSpawnFailure(result)}`;
}
function extractTarGzArchive(archivePath, extractDir, assetName) {
  const failure = runExtractionCommand("tar", ["xzf", archivePath, "-C", extractDir]);
  if (failure) {
    throw new Error(`Failed to extract ${assetName}: ${failure}`);
  }
}
function getWindowsTarCommand() {
  const systemRoot = process.env.SystemRoot ?? process.env.WINDIR;
  if (systemRoot) {
    const systemTar = join9(systemRoot, "System32", "tar.exe");
    if (existsSync7(systemTar)) {
      return systemTar;
    }
  }
  return "tar.exe";
}
function extractZipArchive(archivePath, extractDir, assetName) {
  const failures = [];
  if (platform() === "win32") {
    const tarFailure = runExtractionCommand(getWindowsTarCommand(), ["xf", archivePath, "-C", extractDir]);
    if (!tarFailure)
      return;
    failures.push(tarFailure);
    const script = "& { param($archive, $destination) $ErrorActionPreference = 'Stop'; Expand-Archive -LiteralPath $archive -DestinationPath $destination -Force }";
    const powershellFailure = runExtractionCommand("powershell.exe", [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-Command",
      script,
      archivePath,
      extractDir
    ]);
    if (!powershellFailure)
      return;
    failures.push(powershellFailure);
  } else {
    const unzipFailure = runExtractionCommand("unzip", ["-q", archivePath, "-d", extractDir]);
    if (!unzipFailure)
      return;
    failures.push(unzipFailure);
    const tarFailure = runExtractionCommand("tar", ["xf", archivePath, "-C", extractDir]);
    if (!tarFailure)
      return;
    failures.push(tarFailure);
  }
  throw new Error(`Failed to extract ${assetName}: ${failures.join("; ")}`);
}
async function downloadTool(tool) {
  const config = TOOLS[tool];
  if (!config)
    throw new Error(`Unknown tool: ${tool}`);
  const plat = platform();
  const architecture = arch();
  const version = tool === "fd" && plat === "darwin" && architecture === "x64" ? "10.3.0" : await getLatestVersion(config.repo);
  const assetName = config.getAssetName(version, plat, architecture);
  if (!assetName) {
    throw new Error(`Unsupported platform: ${plat}/${architecture}`);
  }
  mkdirSync5(TOOLS_DIR, { recursive: true });
  const downloadUrl = `https://github.com/${config.repo}/releases/download/${config.tagPrefix}${version}/${assetName}`;
  const archivePath = join9(TOOLS_DIR, assetName);
  const binaryExt = plat === "win32" ? ".exe" : "";
  const binaryPath = join9(TOOLS_DIR, config.binaryName + binaryExt);
  await downloadFile(downloadUrl, archivePath);
  const extractDir = join9(TOOLS_DIR, `extract_tmp_${config.binaryName}_${process.pid}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`);
  mkdirSync5(extractDir, { recursive: true });
  try {
    if (assetName.endsWith(".tar.gz")) {
      extractTarGzArchive(archivePath, extractDir, assetName);
    } else if (assetName.endsWith(".zip")) {
      extractZipArchive(archivePath, extractDir, assetName);
    } else {
      throw new Error(`Unsupported archive format: ${assetName}`);
    }
    const binaryFileName = config.binaryName + binaryExt;
    const extractedDir = join9(extractDir, assetName.replace(/\.(tar\.gz|zip)$/, ""));
    const extractedBinaryCandidates = [join9(extractedDir, binaryFileName), join9(extractDir, binaryFileName)];
    let extractedBinary = extractedBinaryCandidates.find((candidate) => existsSync7(candidate));
    if (!extractedBinary) {
      extractedBinary = findBinaryRecursively(extractDir, binaryFileName) ?? void 0;
    }
    if (extractedBinary) {
      renameSync(extractedBinary, binaryPath);
    } else {
      throw new Error(`Binary not found in archive: expected ${binaryFileName} under ${extractDir}`);
    }
    if (plat !== "win32") {
      chmodSync(binaryPath, 493);
    }
  } finally {
    rmSync(archivePath, { force: true });
    rmSync(extractDir, { recursive: true, force: true });
  }
  return binaryPath;
}
var TERMUX_PACKAGES = {
  fd: "fd",
  rg: "ripgrep"
};
async function ensureTool(tool, onStatus) {
  const existingPath = getToolPath(tool);
  if (existingPath) {
    return existingPath;
  }
  const config = TOOLS[tool];
  if (!config)
    return void 0;
  if (isOfflineModeEnabled()) {
    onStatus?.({ type: "warning", message: `${config.name} not found. Offline mode enabled, skipping download.` });
    return void 0;
  }
  if (platform() === "android") {
    const pkgName = TERMUX_PACKAGES[tool] ?? tool;
    onStatus?.({ type: "warning", message: `${config.name} not found. Install with: pkg install ${pkgName}` });
    return void 0;
  }
  onStatus?.({ type: "info", message: `${config.name} not found. Downloading...` });
  try {
    const path4 = await downloadTool(tool);
    onStatus?.({ type: "info", message: `${config.name} installed to ${path4}` });
    return path4;
  } catch (e) {
    const messages = [];
    for (let current = e, depth = 0; current instanceof Error && depth < 5; current = current.cause, depth++) {
      if (!messages.includes(current.message))
        messages.push(current.message);
    }
    onStatus?.({
      type: "warning",
      message: `Failed to download ${config.name}: ${messages.length > 0 ? messages.join(": ") : String(e)}`
    });
    return void 0;
  }
}

// target/vendor-pi/dist/core/tools/renderers/find.js
function formatFindCall(args, theme31) {
  const pattern = str(args?.pattern);
  const rawPath = str(args?.path);
  const path4 = rawPath !== null ? shortenPath(rawPath || ".") : null;
  const limit = args?.limit;
  const invalidArg = invalidArgText(theme31);
  let text = theme31.fg("toolTitle", theme31.bold("find")) + " " + (pattern === null ? invalidArg : theme31.fg("accent", pattern || "")) + theme31.fg("toolOutput", ` in ${path4 === null ? invalidArg : path4}`);
  if (limit !== void 0) {
    text += theme31.fg("toolOutput", ` (limit ${limit})`);
  }
  return text;
}
function formatFindResult(result, options, theme31, showImages) {
  const output = getTextOutput(result, showImages).trim();
  let text = "";
  if (output) {
    const lines = output.split("\n");
    const maxLines = options.expanded ? lines.length : 20;
    const displayLines = lines.slice(0, maxLines);
    const remaining = lines.length - maxLines;
    text += `
${displayLines.map((line) => theme31.fg("toolOutput", line)).join("\n")}`;
    if (remaining > 0) {
      text += `${theme31.fg("muted", `
... (${remaining} more lines,`)} ${keyHint("app.tools.expand", "to expand")}${theme31.fg("muted", ")")}`;
    }
  }
  const resultLimit = result.details?.resultLimitReached;
  const truncation = result.details?.truncation;
  if (resultLimit || truncation?.truncated) {
    const warnings = [];
    if (resultLimit)
      warnings.push(`${resultLimit} results limit`);
    if (truncation?.truncated)
      warnings.push(`${formatSize(truncation.maxBytes ?? DEFAULT_MAX_BYTES)} limit`);
    text += `
${theme31.fg("warning", `[Truncated: ${warnings.join(", ")}]`)}`;
  }
  return text;
}
var findRenderers = {
  renderCall(args, theme31, context) {
    const text = context.lastComponent ?? new Text("", 0, 0);
    text.setText(formatFindCall(args, theme31));
    return text;
  },
  renderResult(result, options, theme31, context) {
    const text = context.lastComponent ?? new Text("", 0, 0);
    text.setText(formatFindResult(result, options, theme31, context.showImages));
    return text;
  }
};

// target/vendor-pi/dist/core/tools/find.js
function relativizeFindResultPath(resultPath, searchPath, pathModule = path2) {
  const hadTrailingSeparator = resultPath.endsWith(pathModule.sep) || pathModule.sep === "\\" && resultPath.endsWith("/");
  const relativePath = pathModule.isAbsolute(resultPath) ? pathModule.relative(searchPath, resultPath) : resultPath;
  const posixPath = relativePath.split(pathModule.sep).join("/");
  return hadTrailingSeparator && !posixPath.endsWith("/") ? `${posixPath}/` : posixPath;
}
var findSchema = typebox_exports.Object({
  pattern: typebox_exports.String({
    description: "Glob pattern to match files, e.g. '*.ts', '**/*.json', or 'src/**/*.spec.ts'"
  }),
  path: typebox_exports.Optional(typebox_exports.String({ description: "Directory to search in (default: current directory)" })),
  limit: typebox_exports.Optional(typebox_exports.Number({ description: "Maximum number of results (default: 1000)" }))
});
var findToolSystemPromptContribution = {
  snippet: "Find files by glob pattern (respects .gitignore)",
  guidelines: []
};
var DEFAULT_LIMIT = 1e3;
var defaultFindOperations = {
  exists: pathExists,
  // This is a placeholder. Actual fd execution happens in execute() when no custom glob is provided.
  glob: () => []
};
function createFindToolDefinition(cwd, options) {
  const customOps = options?.operations;
  return {
    name: "find",
    label: "find",
    description: `Search for files by glob pattern. Returns matching file paths relative to the search directory. Respects .gitignore. Output is truncated to ${DEFAULT_LIMIT} results or ${DEFAULT_MAX_BYTES / 1024}KB (whichever is hit first).`,
    promptSnippet: findToolSystemPromptContribution.snippet,
    parameters: findSchema,
    async execute(_toolCallId, { pattern, path: searchDir, limit }, signal, _onUpdate, ctx) {
      return new Promise((resolve5, reject) => {
        if (signal?.aborted) {
          reject(new Error("Operation aborted"));
          return;
        }
        let settled = false;
        let stopChild;
        const settle = (fn) => {
          if (settled)
            return;
          settled = true;
          signal?.removeEventListener("abort", onAbort);
          stopChild = void 0;
          fn();
        };
        const onAbort = () => {
          stopChild?.();
          settle(() => reject(new Error("Operation aborted")));
        };
        signal?.addEventListener("abort", onAbort, { once: true });
        (async () => {
          try {
            const searchPath = resolveToCwd(searchDir || ".", ctx?.cwd || cwd);
            const effectiveLimit = limit ?? DEFAULT_LIMIT;
            const ops = customOps ?? defaultFindOperations;
            if (customOps?.glob) {
              if (!await ops.exists(searchPath)) {
                settle(() => reject(new Error(`Path not found: ${searchPath}`)));
                return;
              }
              if (signal?.aborted) {
                settle(() => reject(new Error("Operation aborted")));
                return;
              }
              const results = await ops.glob(pattern, searchPath, {
                ignore: ["**/node_modules/**", "**/.git/**"],
                limit: effectiveLimit
              });
              if (signal?.aborted) {
                settle(() => reject(new Error("Operation aborted")));
                return;
              }
              if (results.length === 0) {
                settle(() => resolve5({
                  content: [{ type: "text", text: "No files found matching pattern" }],
                  details: void 0
                }));
                return;
              }
              const relativized = results.map((p) => relativizeFindResultPath(p, searchPath));
              const resultLimitReached = relativized.length >= effectiveLimit;
              const rawOutput = relativized.join("\n");
              const truncation = truncateHead(rawOutput, { maxLines: Number.MAX_SAFE_INTEGER });
              let resultOutput = truncation.content;
              const details = {};
              const notices = [];
              if (resultLimitReached) {
                notices.push(`${effectiveLimit} results limit reached`);
                details.resultLimitReached = effectiveLimit;
              }
              if (truncation.truncated) {
                notices.push(`${formatSize(DEFAULT_MAX_BYTES)} limit reached`);
                details.truncation = truncation;
              }
              if (notices.length > 0) {
                resultOutput += `

[${notices.join(". ")}]`;
              }
              settle(() => resolve5({
                content: [{ type: "text", text: resultOutput }],
                details: Object.keys(details).length > 0 ? details : void 0
              }));
              return;
            }
            const fdPath = await ensureTool("fd");
            if (signal?.aborted) {
              settle(() => reject(new Error("Operation aborted")));
              return;
            }
            if (!fdPath) {
              settle(() => reject(new Error("fd is not available and could not be downloaded")));
              return;
            }
            const args = ["--glob", "--color=never", "--hidden"];
            let insideGitRepo = false;
            for (let current = searchPath; ; ) {
              if (await pathExists(path2.join(current, ".git"))) {
                insideGitRepo = true;
                break;
              }
              const parent = path2.dirname(current);
              if (parent === current)
                break;
              current = parent;
            }
            if (!insideGitRepo)
              args.push("--no-require-git");
            args.push("--max-results", String(effectiveLimit));
            let effectivePattern = pattern;
            if (pattern.includes("/")) {
              args.push("--full-path");
              if (!pattern.startsWith("/") && !pattern.startsWith("**/") && pattern !== "**") {
                effectivePattern = `**/${pattern}`;
              }
              if (process.platform === "win32")
                effectivePattern = effectivePattern.replaceAll("/", String.raw`[/\\]`);
            }
            args.push("--", effectivePattern, searchPath);
            const child = spawn3(fdPath, args, { stdio: ["ignore", "pipe", "pipe"] });
            const rl = createInterface2({ input: child.stdout });
            let stderr = "";
            const lines = [];
            stopChild = () => {
              if (!child.killed) {
                child.kill();
              }
            };
            const cleanup = () => {
              rl.close();
            };
            child.stderr?.on("data", (chunk) => {
              stderr += chunk.toString();
            });
            rl.on("line", (line) => {
              lines.push(line);
            });
            child.on("error", (error) => {
              cleanup();
              settle(() => reject(new Error(`Failed to run fd: ${error.message}`)));
            });
            child.on("close", (code) => {
              cleanup();
              if (signal?.aborted) {
                settle(() => reject(new Error("Operation aborted")));
                return;
              }
              const output = lines.join("\n");
              if (code !== 0) {
                const errorMsg = stderr.trim() || `fd exited with code ${code}`;
                if (!output) {
                  settle(() => reject(new Error(errorMsg)));
                  return;
                }
              }
              if (!output) {
                settle(() => resolve5({
                  content: [{ type: "text", text: "No files found matching pattern" }],
                  details: void 0
                }));
                return;
              }
              const relativized = [];
              for (const rawLine of lines) {
                const line = rawLine.replace(/\r$/, "").trim();
                if (!line)
                  continue;
                relativized.push(relativizeFindResultPath(line, searchPath));
              }
              const resultLimitReached = relativized.length >= effectiveLimit;
              const rawOutput = relativized.join("\n");
              const truncation = truncateHead(rawOutput, { maxLines: Number.MAX_SAFE_INTEGER });
              let resultOutput = truncation.content;
              const details = {};
              const notices = [];
              if (resultLimitReached) {
                notices.push(`${effectiveLimit} results limit reached. Use limit=${effectiveLimit * 2} for more, or refine pattern`);
                details.resultLimitReached = effectiveLimit;
              }
              if (truncation.truncated) {
                notices.push(`${formatSize(DEFAULT_MAX_BYTES)} limit reached`);
                details.truncation = truncation;
              }
              if (notices.length > 0) {
                resultOutput += `

[${notices.join(". ")}]`;
              }
              settle(() => resolve5({
                content: [{ type: "text", text: resultOutput }],
                details: Object.keys(details).length > 0 ? details : void 0
              }));
            });
          } catch (e) {
            if (signal?.aborted) {
              settle(() => reject(new Error("Operation aborted")));
              return;
            }
            const error = e instanceof Error ? e : new Error(String(e));
            settle(() => reject(error));
          }
        })();
      });
    },
    ...findRenderers
  };
}
function createFindTool(cwd, options) {
  return wrapToolDefinition(createFindToolDefinition(cwd, options));
}

// target/vendor-pi/dist/core/tools/grep.js
import { readFile as fsReadFile2, stat as fsStat } from "node:fs/promises";
import { createInterface as createInterface3 } from "node:readline";
import { spawn as spawn4 } from "child_process";
import path3 from "path";

// target/vendor-pi/dist/core/tools/renderers/grep.js
function formatGrepCall(args, theme31) {
  const pattern = str(args?.pattern);
  const rawPath = str(args?.path);
  const path4 = rawPath !== null ? shortenPath(rawPath || ".") : null;
  const glob = str(args?.glob);
  const limit = args?.limit;
  const invalidArg = invalidArgText(theme31);
  let text = theme31.fg("toolTitle", theme31.bold("grep")) + " " + (pattern === null ? invalidArg : theme31.fg("accent", `/${pattern || ""}/`)) + theme31.fg("toolOutput", ` in ${path4 === null ? invalidArg : path4}`);
  if (glob)
    text += theme31.fg("toolOutput", ` (${glob})`);
  if (limit !== void 0)
    text += theme31.fg("toolOutput", ` limit ${limit}`);
  return text;
}
function formatGrepResult(result, options, theme31, showImages) {
  const output = getTextOutput(result, showImages).trim();
  let text = "";
  if (output) {
    const lines = output.split("\n");
    const maxLines = options.expanded ? lines.length : 15;
    const displayLines = lines.slice(0, maxLines);
    const remaining = lines.length - maxLines;
    text += `
${displayLines.map((line) => theme31.fg("toolOutput", line)).join("\n")}`;
    if (remaining > 0) {
      text += `${theme31.fg("muted", `
... (${remaining} more lines,`)} ${keyHint("app.tools.expand", "to expand")}${theme31.fg("muted", ")")}`;
    }
  }
  const matchLimit = result.details?.matchLimitReached;
  const truncation = result.details?.truncation;
  const linesTruncated = result.details?.linesTruncated;
  if (matchLimit || truncation?.truncated || linesTruncated) {
    const warnings = [];
    if (matchLimit)
      warnings.push(`${matchLimit} matches limit`);
    if (truncation?.truncated)
      warnings.push(`${formatSize(truncation.maxBytes ?? DEFAULT_MAX_BYTES)} limit`);
    if (linesTruncated)
      warnings.push("some lines truncated");
    text += `
${theme31.fg("warning", `[Truncated: ${warnings.join(", ")}]`)}`;
  }
  return text;
}
var grepRenderers = {
  renderCall(args, theme31, context) {
    const text = context.lastComponent ?? new Text("", 0, 0);
    text.setText(formatGrepCall(args, theme31));
    return text;
  },
  renderResult(result, options, theme31, context) {
    const text = context.lastComponent ?? new Text("", 0, 0);
    text.setText(formatGrepResult(result, options, theme31, context.showImages));
    return text;
  }
};

// target/vendor-pi/dist/core/tools/grep.js
var grepSchema = typebox_exports.Object({
  pattern: typebox_exports.String({ description: "Search pattern (regex or literal string)" }),
  path: typebox_exports.Optional(typebox_exports.String({ description: "Directory or file to search (default: current directory)" })),
  glob: typebox_exports.Optional(typebox_exports.String({ description: "Filter files by glob pattern, e.g. '*.ts' or '**/*.spec.ts'" })),
  ignoreCase: typebox_exports.Optional(typebox_exports.Boolean({ description: "Case-insensitive search (default: false)" })),
  literal: typebox_exports.Optional(typebox_exports.Boolean({ description: "Treat pattern as literal string instead of regex (default: false)" })),
  context: typebox_exports.Optional(typebox_exports.Number({ description: "Number of lines to show before and after each match (default: 0)" })),
  limit: typebox_exports.Optional(typebox_exports.Number({ description: "Maximum number of matches to return (default: 100)" }))
});
var grepToolSystemPromptContribution = {
  snippet: "Search file contents for patterns (respects .gitignore)",
  guidelines: []
};
var DEFAULT_LIMIT2 = 100;
var defaultGrepOperations = {
  isDirectory: async (p) => (await fsStat(p)).isDirectory(),
  readFile: (p) => fsReadFile2(p, "utf-8")
};
function createGrepToolDefinition(cwd, options) {
  const customOps = options?.operations;
  return {
    name: "grep",
    label: "grep",
    description: `Search file contents for a pattern. Returns matching lines with file paths and line numbers. Respects .gitignore. Output is truncated to ${DEFAULT_LIMIT2} matches or ${DEFAULT_MAX_BYTES / 1024}KB (whichever is hit first). Long lines are truncated to ${GREP_MAX_LINE_LENGTH} chars.`,
    promptSnippet: grepToolSystemPromptContribution.snippet,
    parameters: grepSchema,
    async execute(_toolCallId, { pattern, path: searchDir, glob, ignoreCase, literal, context, limit }, signal, _onUpdate, ctx) {
      return new Promise((resolve5, reject) => {
        if (signal?.aborted) {
          reject(new Error("Operation aborted"));
          return;
        }
        let settled = false;
        const settle = (fn) => {
          if (!settled) {
            settled = true;
            fn();
          }
        };
        (async () => {
          try {
            const rgPath = await ensureTool("rg");
            if (!rgPath) {
              settle(() => reject(new Error("ripgrep (rg) is not available and could not be downloaded")));
              return;
            }
            const searchPath = resolveToCwd(searchDir || ".", ctx?.cwd || cwd);
            const ops = customOps ?? defaultGrepOperations;
            let isDirectory;
            try {
              isDirectory = await ops.isDirectory(searchPath);
            } catch {
              settle(() => reject(new Error(`Path not found: ${searchPath}`)));
              return;
            }
            const contextValue = context && context > 0 ? context : 0;
            const effectiveLimit = Math.max(1, limit ?? DEFAULT_LIMIT2);
            const formatPath = (filePath) => {
              if (isDirectory) {
                const relative5 = path3.relative(searchPath, filePath);
                if (relative5 && !relative5.startsWith("..")) {
                  return relative5.replace(/\\/g, "/");
                }
              }
              return path3.basename(filePath);
            };
            const fileCache = /* @__PURE__ */ new Map();
            const getFileLines = async (filePath) => {
              let lines = fileCache.get(filePath);
              if (!lines) {
                try {
                  const content = await ops.readFile(filePath);
                  lines = content.replace(/\r\n/g, "\n").replace(/\r/g, "\n").split("\n");
                } catch {
                  lines = [];
                }
                fileCache.set(filePath, lines);
              }
              return lines;
            };
            const args = ["--json", "--line-number", "--color=never", "--hidden"];
            if (ignoreCase)
              args.push("--ignore-case");
            if (literal)
              args.push("--fixed-strings");
            if (glob)
              args.push("--glob", glob);
            args.push("--", pattern, searchPath);
            const child = spawn4(rgPath, args, { stdio: ["ignore", "pipe", "pipe"] });
            const rl = createInterface3({ input: child.stdout });
            let stderr = "";
            let matchCount = 0;
            let matchLimitReached = false;
            let linesTruncated = false;
            let aborted = false;
            let killedDueToLimit = false;
            const outputLines = [];
            const cleanup = () => {
              rl.close();
              signal?.removeEventListener("abort", onAbort);
            };
            const stopChild = (dueToLimit = false) => {
              if (!child.killed) {
                killedDueToLimit = dueToLimit;
                child.kill();
              }
            };
            const onAbort = () => {
              aborted = true;
              stopChild();
            };
            signal?.addEventListener("abort", onAbort, { once: true });
            child.stderr?.on("data", (chunk) => {
              stderr += chunk.toString();
            });
            const formatBlock = async (filePath, lineNumber) => {
              const relativePath = formatPath(filePath);
              const lines = await getFileLines(filePath);
              if (!lines.length)
                return [`${relativePath}:${lineNumber}: (unable to read file)`];
              const block = [];
              const start = contextValue > 0 ? Math.max(1, lineNumber - contextValue) : lineNumber;
              const end = contextValue > 0 ? Math.min(lines.length, lineNumber + contextValue) : lineNumber;
              for (let current = start; current <= end; current++) {
                const lineText = lines[current - 1] ?? "";
                const sanitized = lineText.replace(/\r/g, "");
                const isMatchLine = current === lineNumber;
                const { text: truncatedText, wasTruncated } = truncateLine(sanitized);
                if (wasTruncated)
                  linesTruncated = true;
                if (isMatchLine)
                  block.push(`${relativePath}:${current}: ${truncatedText}`);
                else
                  block.push(`${relativePath}-${current}- ${truncatedText}`);
              }
              return block;
            };
            const matches = [];
            rl.on("line", (line) => {
              if (!line.trim() || matchCount >= effectiveLimit)
                return;
              let event;
              try {
                event = JSON.parse(line);
              } catch {
                return;
              }
              if (event.type === "match") {
                matchCount++;
                const filePath = event.data?.path?.text;
                const lineNumber = event.data?.line_number;
                const lineText = event.data?.lines?.text;
                if (filePath && typeof lineNumber === "number")
                  matches.push({ filePath, lineNumber, lineText });
                if (matchCount >= effectiveLimit) {
                  matchLimitReached = true;
                  stopChild(true);
                }
              }
            });
            child.on("error", (error) => {
              cleanup();
              settle(() => reject(new Error(`Failed to run ripgrep: ${error.message}`)));
            });
            child.on("close", async (code) => {
              cleanup();
              if (aborted) {
                settle(() => reject(new Error("Operation aborted")));
                return;
              }
              if (!killedDueToLimit && code !== 0 && code !== 1) {
                const errorMsg = stderr.trim() || `ripgrep exited with code ${code}`;
                settle(() => reject(new Error(errorMsg)));
                return;
              }
              if (matchCount === 0) {
                settle(() => resolve5({ content: [{ type: "text", text: "No matches found" }], details: void 0 }));
                return;
              }
              for (const match2 of matches) {
                if (contextValue === 0 && match2.lineText !== void 0) {
                  const relativePath = formatPath(match2.filePath);
                  const sanitized = match2.lineText.replace(/\r\n/g, "\n").replace(/\r/g, "").replace(/\n$/, "");
                  const { text: truncatedText, wasTruncated } = truncateLine(sanitized);
                  if (wasTruncated)
                    linesTruncated = true;
                  outputLines.push(`${relativePath}:${match2.lineNumber}: ${truncatedText}`);
                } else {
                  const block = await formatBlock(match2.filePath, match2.lineNumber);
                  outputLines.push(...block);
                }
              }
              const rawOutput = outputLines.join("\n");
              const truncation = truncateHead(rawOutput, { maxLines: Number.MAX_SAFE_INTEGER });
              let output = truncation.content;
              const details = {};
              const notices = [];
              if (matchLimitReached) {
                notices.push(`${effectiveLimit} matches limit reached. Use limit=${effectiveLimit * 2} for more, or refine pattern`);
                details.matchLimitReached = effectiveLimit;
              }
              if (truncation.truncated) {
                notices.push(`${formatSize(DEFAULT_MAX_BYTES)} limit reached`);
                details.truncation = truncation;
              }
              if (linesTruncated) {
                notices.push(`Some lines truncated to ${GREP_MAX_LINE_LENGTH} chars. Use read tool to see full lines`);
                details.linesTruncated = true;
              }
              if (notices.length > 0)
                output += `

[${notices.join(". ")}]`;
              settle(() => resolve5({
                content: [{ type: "text", text: output }],
                details: Object.keys(details).length > 0 ? details : void 0
              }));
            });
          } catch (err) {
            settle(() => reject(err));
          }
        })();
      });
    },
    ...grepRenderers
  };
}
function createGrepTool(cwd, options) {
  return wrapToolDefinition(createGrepToolDefinition(cwd, options));
}

// target/vendor-pi/dist/core/tools/ls.js
import { readdir as fsReaddir, stat as fsStat2 } from "node:fs/promises";
import nodePath from "path";

// target/vendor-pi/dist/core/tools/renderers/ls.js
function formatLsCall(args, theme31, cwd) {
  const limit = args?.limit;
  const pathDisplay = renderToolPath(str(args?.path), theme31, cwd, { emptyFallback: "." });
  let text = `${theme31.fg("toolTitle", theme31.bold("ls"))} ${pathDisplay}`;
  if (limit !== void 0) {
    text += theme31.fg("toolOutput", ` (limit ${limit})`);
  }
  return text;
}
function formatLsResult(result, options, theme31, showImages) {
  const output = getTextOutput(result, showImages).trim();
  let text = "";
  if (output) {
    const lines = output.split("\n");
    const maxLines = options.expanded ? lines.length : 20;
    const displayLines = lines.slice(0, maxLines);
    const remaining = lines.length - maxLines;
    text += `
${displayLines.map((line) => theme31.fg("toolOutput", line)).join("\n")}`;
    if (remaining > 0) {
      text += `${theme31.fg("muted", `
... (${remaining} more lines,`)} ${keyHint("app.tools.expand", "to expand")}${theme31.fg("muted", ")")}`;
    }
  }
  const entryLimit = result.details?.entryLimitReached;
  const truncation = result.details?.truncation;
  if (entryLimit || truncation?.truncated) {
    const warnings = [];
    if (entryLimit)
      warnings.push(`${entryLimit} entries limit`);
    if (truncation?.truncated)
      warnings.push(`${formatSize(truncation.maxBytes ?? DEFAULT_MAX_BYTES)} limit`);
    text += `
${theme31.fg("warning", `[Truncated: ${warnings.join(", ")}]`)}`;
  }
  return text;
}
var lsRenderers = {
  renderCall(args, theme31, context) {
    const text = context.lastComponent ?? new Text("", 0, 0);
    text.setText(formatLsCall(args, theme31, context.cwd));
    return text;
  },
  renderResult(result, options, theme31, context) {
    const text = context.lastComponent ?? new Text("", 0, 0);
    text.setText(formatLsResult(result, options, theme31, context.showImages));
    return text;
  }
};

// target/vendor-pi/dist/core/tools/ls.js
var lsSchema = typebox_exports.Object({
  path: typebox_exports.Optional(typebox_exports.String({ description: "Directory to list (default: current directory)" })),
  limit: typebox_exports.Optional(typebox_exports.Number({ description: "Maximum number of entries to return (default: 500)" }))
});
var lsToolSystemPromptContribution = {
  snippet: "List directory contents",
  guidelines: []
};
var DEFAULT_LIMIT3 = 500;
var defaultLsOperations = {
  exists: pathExists,
  stat: fsStat2,
  readdir: fsReaddir
};
function createLsToolDefinition(cwd, options) {
  const ops = options?.operations ?? defaultLsOperations;
  return {
    name: "ls",
    label: "ls",
    description: `List directory contents. Returns entries sorted alphabetically, with '/' suffix for directories. Includes dotfiles. Output is truncated to ${DEFAULT_LIMIT3} entries or ${DEFAULT_MAX_BYTES / 1024}KB (whichever is hit first).`,
    promptSnippet: lsToolSystemPromptContribution.snippet,
    parameters: lsSchema,
    async execute(_toolCallId, { path: path4, limit }, signal, _onUpdate, ctx) {
      return new Promise((resolve5, reject) => {
        if (signal?.aborted) {
          reject(new Error("Operation aborted"));
          return;
        }
        const onAbort = () => reject(new Error("Operation aborted"));
        signal?.addEventListener("abort", onAbort, { once: true });
        (async () => {
          try {
            const dirPath = resolveToCwd(path4 || ".", ctx?.cwd || cwd);
            const effectiveLimit = limit ?? DEFAULT_LIMIT3;
            if (!await ops.exists(dirPath)) {
              reject(new Error(`Path not found: ${dirPath}`));
              return;
            }
            const stat2 = await ops.stat(dirPath);
            if (!stat2.isDirectory()) {
              reject(new Error(`Not a directory: ${dirPath}`));
              return;
            }
            let entries;
            try {
              entries = await ops.readdir(dirPath);
            } catch (e) {
              reject(new Error(`Cannot read directory: ${e.message}`));
              return;
            }
            entries.sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()));
            const results = [];
            let entryLimitReached = false;
            for (const entry of entries) {
              if (results.length >= effectiveLimit) {
                entryLimitReached = true;
                break;
              }
              const fullPath = nodePath.join(dirPath, entry);
              let suffix = "";
              try {
                const entryStat = await ops.stat(fullPath);
                if (entryStat.isDirectory())
                  suffix = "/";
              } catch {
                continue;
              }
              results.push(entry + suffix);
            }
            signal?.removeEventListener("abort", onAbort);
            if (results.length === 0) {
              resolve5({ content: [{ type: "text", text: "(empty directory)" }], details: void 0 });
              return;
            }
            const rawOutput = results.join("\n");
            const truncation = truncateHead(rawOutput, { maxLines: Number.MAX_SAFE_INTEGER });
            let output = truncation.content;
            const details = {};
            const notices = [];
            if (entryLimitReached) {
              notices.push(`${effectiveLimit} entries limit reached. Use limit=${effectiveLimit * 2} for more`);
              details.entryLimitReached = effectiveLimit;
            }
            if (truncation.truncated) {
              notices.push(`${formatSize(DEFAULT_MAX_BYTES)} limit reached`);
              details.truncation = truncation;
            }
            if (notices.length > 0) {
              output += `

[${notices.join(". ")}]`;
            }
            resolve5({
              content: [{ type: "text", text: output }],
              details: Object.keys(details).length > 0 ? details : void 0
            });
          } catch (e) {
            signal?.removeEventListener("abort", onAbort);
            reject(e);
          }
        })();
      });
    },
    ...lsRenderers
  };
}
function createLsTool(cwd, options) {
  return wrapToolDefinition(createLsToolDefinition(cwd, options));
}

// target/vendor-pi/dist/core/tools/powershell.js
var UTF8_OUTPUT_PREFIX = "try { [Console]::OutputEncoding=[System.Text.Encoding]::UTF8 } catch {}\n";
var powershellToolSystemPromptContribution = {
  snippet: "Execute PowerShell commands",
  guidelines: ["You can inspect PI_* environment variables for current model and session details."]
};
function createLocalPowerShellOperations() {
  const operations = createLocalShellOperations("PowerShell", getPowerShellConfig);
  return {
    exec: (command, cwd, options) => operations.exec(`${UTF8_OUTPUT_PREFIX}${command}`, cwd, options)
  };
}
var powershellToolConfig = {
  name: "powershell",
  label: "powershell",
  shellName: "PowerShell",
  prompt: "PS>",
  promptSnippet: powershellToolSystemPromptContribution.snippet,
  promptGuidelines: powershellToolSystemPromptContribution.guidelines,
  tempFilePrefix: "pi-powershell"
};
function createPowerShellToolDefinition(cwd, options) {
  return createShellToolDefinition(cwd, powershellToolConfig, {
    ...options,
    operations: options?.operations ?? createLocalPowerShellOperations()
  });
}
function createPowerShellTool(cwd, options) {
  const definition = createPowerShellToolDefinition(cwd, options);
  const tool = wrapToolDefinition(definition);
  Object.assign(tool, {
    promptSnippet: definition.promptSnippet,
    promptGuidelines: definition.promptGuidelines
  });
  return tool;
}

// target/vendor-pi/dist/core/tools/read.js
import { constants as constants5 } from "fs";
import { access as fsAccess3, readFile as fsReadFile3 } from "fs/promises";

// target/vendor-pi/dist/utils/image-convert.js
var image_convert_exports = {};
__reExport(image_convert_exports, host_star4);
import * as host_star4 from "yapi:pi/host";

// target/vendor-pi/dist/utils/image-resize.js
var image_resize_exports = {};
__reExport(image_resize_exports, host_star5);
import * as host_star5 from "yapi:pi/host";

// target/vendor-pi/dist/utils/image-process.js
function baseMimeType(mimeType) {
  return mimeType.split(";")[0]?.trim().toLowerCase() ?? mimeType.toLowerCase();
}
function normalizeSupportedImageMimeType(mimeType) {
  switch (baseMimeType(mimeType)) {
    case "image/png":
      return "image/png";
    case "image/jpeg":
    case "image/jpg":
      return "image/jpeg";
    case "image/gif":
      return "image/gif";
    case "image/webp":
      return "image/webp";
    default:
      return null;
  }
}
async function normalizeImage(bytes, mimeType) {
  const normalizedMimeType = normalizeSupportedImageMimeType(mimeType);
  if (normalizedMimeType) {
    return { bytes, mimeType: normalizedMimeType };
  }
  const pngBytes = await (0, image_convert_exports.convertImageBytesToPng)(bytes);
  if (!pngBytes) {
    return null;
  }
  return {
    bytes: pngBytes,
    mimeType: "image/png",
    convertedFrom: baseMimeType(mimeType)
  };
}
function conversionHint(from, to) {
  if (!from || from === to)
    return void 0;
  return `[Image converted from ${from} to ${to}.]`;
}
async function processImage(bytes, mimeType, options) {
  const autoResizeImages = options?.autoResizeImages ?? true;
  const normalized = await normalizeImage(bytes, mimeType);
  if (!normalized) {
    return {
      ok: false,
      message: "[Image omitted: could not be converted to a supported inline image format.]"
    };
  }
  if (autoResizeImages) {
    const resized = await (0, image_resize_exports.resizeImage)(normalized.bytes, normalized.mimeType, options?.resizeOptions);
    if (!resized) {
      return {
        ok: false,
        message: "[Image omitted: could not be resized below the inline image size limit.]"
      };
    }
    const hints2 = [];
    const convertedHint2 = conversionHint(normalized.convertedFrom, resized.mimeType);
    if (convertedHint2)
      hints2.push(convertedHint2);
    const dimensionNote = (0, image_resize_exports.formatDimensionNote)(resized);
    if (dimensionNote)
      hints2.push(dimensionNote);
    return {
      ok: true,
      data: resized.data,
      mimeType: resized.mimeType,
      hints: hints2
    };
  }
  const hints = [];
  const convertedHint = conversionHint(normalized.convertedFrom, normalized.mimeType);
  if (convertedHint)
    hints.push(convertedHint);
  return {
    ok: true,
    data: Buffer.from(normalized.bytes).toString("base64"),
    mimeType: normalized.mimeType,
    hints
  };
}

// target/vendor-pi/dist/core/tools/renderers/read.js
import { basename as basename3, dirname as dirname5, isAbsolute as isAbsolute2, relative as relative3, resolve as resolvePath2, sep as sep4 } from "node:path";
var COMPACT_RESOURCE_FILE_NAMES = /* @__PURE__ */ new Set(["AGENTS.override.md", "AGENTS.md", "AGENTS.MD", "CLAUDE.md", "CLAUDE.MD"]);
function formatReadLineRange(args, theme31) {
  if (args?.offset == null && args?.limit == null)
    return "";
  const startLine = args.offset ?? 1;
  const endLine = args.limit != null ? startLine + args.limit - 1 : "";
  return theme31.fg("warning", `:${startLine}${endLine ? `-${endLine}` : ""}`);
}
function formatReadCall(args, theme31, cwd) {
  const pathDisplay = renderToolPath(str(args?.file_path ?? args?.path), theme31, cwd);
  return `${theme31.fg("toolTitle", theme31.bold("read"))} ${pathDisplay}${formatReadLineRange(args, theme31)}`;
}
function trimTrailingEmptyLines(lines) {
  let end = lines.length;
  while (end > 0 && lines[end - 1] === "") {
    end--;
  }
  return lines.slice(0, end);
}
function toPosixPath2(filePath) {
  return filePath.split(sep4).join("/");
}
function getPiDocsClassification(absolutePath) {
  const packageRoot = dirname5((0, config_exports.getReadmePath)());
  const relativePath = relative3(resolvePath2(packageRoot), resolvePath2(absolutePath));
  if (relativePath === "" || relativePath === ".." || relativePath.startsWith(`..${sep4}`) || isAbsolute2(relativePath)) {
    return void 0;
  }
  const label = toPosixPath2(relativePath);
  if (label === "README.md" || label.startsWith("docs/") || label.startsWith("examples/")) {
    return { kind: "docs", label };
  }
  return void 0;
}
function getCompactReadClassification(args, cwd) {
  const rawPath = str(args?.file_path ?? args?.path);
  if (!rawPath)
    return void 0;
  const absolutePath = resolveToCwd(rawPath, cwd);
  const fileName = basename3(absolutePath);
  if (fileName === "SKILL.md") {
    return { kind: "skill", label: basename3(dirname5(absolutePath)) || fileName };
  }
  const docsClassification = getPiDocsClassification(absolutePath);
  if (docsClassification)
    return docsClassification;
  if (COMPACT_RESOURCE_FILE_NAMES.has(fileName)) {
    return { kind: "resource", label: formatPathRelativeToCwdOrAbsolute(absolutePath, cwd) };
  }
  return void 0;
}
function formatCompactReadCall(classification, args, theme31) {
  const expandHint = theme31.fg("dim", ` (${keyText("app.tools.expand")} to expand)`);
  if (classification.kind === "skill") {
    return theme31.fg("customMessageLabel", `\x1B[1m[skill]\x1B[22m `) + theme31.fg("customMessageText", classification.label) + formatReadLineRange(args, theme31) + expandHint;
  }
  return theme31.fg("toolTitle", theme31.bold(`read ${classification.kind}`)) + " " + theme31.fg("accent", classification.label) + formatReadLineRange(args, theme31) + expandHint;
}
function formatReadResult(args, result, options, theme31, showImages, _cwd, isError) {
  if (!options.expanded && !isError) {
    return "";
  }
  const rawPath = str(args?.file_path ?? args?.path);
  const output = getTextOutput(result, showImages);
  const lang = !isError && rawPath ? (0, theme_exports.getLanguageFromPath)(rawPath) : void 0;
  const renderedLines = lang ? (0, theme_exports.highlightCode)(replaceTabs(output), lang) : output.split("\n");
  const lines = trimTrailingEmptyLines(renderedLines);
  const maxLines = options.expanded ? lines.length : 10;
  const displayLines = lines.slice(0, maxLines);
  const remaining = lines.length - maxLines;
  let text = `
${displayLines.map((line) => lang ? replaceTabs(line) : theme31.fg("toolOutput", replaceTabs(line))).join("\n")}`;
  if (remaining > 0) {
    text += `${theme31.fg("muted", `
... (${remaining} more lines,`)} ${keyHint("app.tools.expand", "to expand")}${theme31.fg("muted", ")")}`;
  }
  const truncation = result.details?.truncation;
  if (truncation?.truncated) {
    if (truncation.firstLineExceedsLimit) {
      text += `
${theme31.fg("warning", `[First line exceeds ${formatSize(truncation.maxBytes ?? DEFAULT_MAX_BYTES)} limit]`)}`;
    } else if (truncation.truncatedBy === "lines") {
      text += `
${theme31.fg("warning", `[Truncated: showing ${truncation.outputLines} of ${truncation.totalLines} lines (${truncation.maxLines ?? DEFAULT_MAX_LINES} line limit)]`)}`;
    } else {
      text += `
${theme31.fg("warning", `[Truncated: ${truncation.outputLines} lines shown (${formatSize(truncation.maxBytes ?? DEFAULT_MAX_BYTES)} limit)]`)}`;
    }
  }
  return text;
}
var readRenderers = {
  renderCall(rawArgs, theme31, context) {
    const args = rawArgs;
    const text = context.lastComponent ?? new Text("", 0, 0);
    const classification = !context.expanded ? getCompactReadClassification(args, context.cwd) : void 0;
    text.setText(classification ? formatCompactReadCall(classification, args, theme31) : formatReadCall(args, theme31, context.cwd));
    return text;
  },
  renderResult(result, options, theme31, context) {
    const text = context.lastComponent ?? new Text("", 0, 0);
    text.setText(formatReadResult(context.args, result, options, theme31, context.showImages, context.cwd, context.isError));
    return text;
  }
};

// target/vendor-pi/dist/core/tools/read.js
var readSchema = typebox_exports.Object({
  path: typebox_exports.String({ description: "Path to the file to read (relative or absolute)" }),
  offset: typebox_exports.Optional(typebox_exports.Number({ description: "Line number to start reading from (1-indexed)" })),
  limit: typebox_exports.Optional(typebox_exports.Number({ description: "Maximum number of lines to read" }))
});
var readToolSystemPromptContribution = {
  snippet: "Read file contents",
  guidelines: ["Use read to examine files instead of cat or sed."]
};
var defaultReadOperations = {
  readFile: (path4) => fsReadFile3(path4),
  access: (path4) => fsAccess3(path4, constants5.R_OK),
  detectImageMimeType: detectSupportedImageMimeTypeFromFile
};
function getNonVisionImageNote(model) {
  if (!model || model.input.includes("image")) {
    return void 0;
  }
  return "[Current model does not support images. The image will be omitted from this request.]";
}
function createReadToolDefinition(cwd, options) {
  const autoResizeImages = options?.autoResizeImages ?? true;
  const fallbackResizeOptions = options?.resizeOptions;
  const ops = options?.operations ?? defaultReadOperations;
  return {
    name: "read",
    label: "read",
    description: `Read the contents of a file. Supports text files and images (jpg, png, gif, webp, bmp). Images are sent as attachments. For text files, output is truncated to ${DEFAULT_MAX_LINES} lines or ${DEFAULT_MAX_BYTES / 1024}KB (whichever is hit first). Use offset/limit for large files. When you need the full file, continue with offset until complete.`,
    promptSnippet: readToolSystemPromptContribution.snippet,
    promptGuidelines: [...readToolSystemPromptContribution.guidelines],
    parameters: readSchema,
    constrainedSampling: { type: "json_schema", strict: "prefer" },
    async execute(_toolCallId, { path: path4, offset, limit }, signal, _onUpdate, ctx) {
      return new Promise((resolve5, reject) => {
        if (signal?.aborted) {
          reject(new Error("Operation aborted"));
          return;
        }
        let aborted = false;
        const onAbort = () => {
          aborted = true;
          reject(new Error("Operation aborted"));
        };
        signal?.addEventListener("abort", onAbort, { once: true });
        (async () => {
          try {
            const absolutePath = await resolveReadPathAsync(path4, ctx?.cwd || cwd);
            if (aborted)
              return;
            await ops.access(absolutePath);
            if (aborted)
              return;
            const mimeType = ops.detectImageMimeType ? await ops.detectImageMimeType(absolutePath) : void 0;
            let content;
            let details;
            const nonVisionImageNote = getNonVisionImageNote(ctx?.model);
            if (mimeType) {
              const buffer = await ops.readFile(absolutePath);
              const processed = await processImage(buffer, mimeType, {
                autoResizeImages,
                resizeOptions: ctx?.model?.inputLimits?.images?.resize ?? fallbackResizeOptions
              });
              if (!processed.ok) {
                let textNote = `Read image file [${mimeType}]
${processed.message}`;
                if (nonVisionImageNote)
                  textNote += `
${nonVisionImageNote}`;
                content = [{ type: "text", text: textNote }];
              } else {
                let textNote = `Read image file [${processed.mimeType}]`;
                if (processed.hints.length > 0)
                  textNote += `
${processed.hints.join("\n")}`;
                if (nonVisionImageNote)
                  textNote += `
${nonVisionImageNote}`;
                content = [
                  { type: "text", text: textNote },
                  { type: "image", data: processed.data, mimeType: processed.mimeType }
                ];
              }
            } else {
              const buffer = await ops.readFile(absolutePath);
              const textContent = buffer.toString("utf-8");
              const allLines = textContent.split("\n");
              const totalFileLines = allLines.length;
              const startLine = offset ? Math.max(0, offset - 1) : 0;
              const startLineDisplay = startLine + 1;
              if (startLine >= allLines.length) {
                throw new Error(`Offset ${offset} is beyond end of file (${allLines.length} lines total)`);
              }
              let selectedContent;
              let userLimitedLines;
              if (limit !== void 0) {
                const endLine = Math.min(startLine + limit, allLines.length);
                selectedContent = allLines.slice(startLine, endLine).join("\n");
                userLimitedLines = endLine - startLine;
              } else {
                selectedContent = allLines.slice(startLine).join("\n");
              }
              const truncation = truncateHead(selectedContent);
              let outputText;
              if (truncation.firstLineExceedsLimit) {
                const firstLineSize = formatSize(Buffer.byteLength(allLines[startLine], "utf-8"));
                outputText = `[Line ${startLineDisplay} is ${firstLineSize}, exceeds ${formatSize(DEFAULT_MAX_BYTES)} limit. Use bash: sed -n '${startLineDisplay}p' ${path4} | head -c ${DEFAULT_MAX_BYTES}]`;
                details = { truncation };
              } else if (truncation.truncated) {
                const endLineDisplay = startLineDisplay + truncation.outputLines - 1;
                const nextOffset = endLineDisplay + 1;
                outputText = truncation.content;
                if (truncation.truncatedBy === "lines") {
                  outputText += `

[Showing lines ${startLineDisplay}-${endLineDisplay} of ${totalFileLines}. Use offset=${nextOffset} to continue.]`;
                } else {
                  outputText += `

[Showing lines ${startLineDisplay}-${endLineDisplay} of ${totalFileLines} (${formatSize(DEFAULT_MAX_BYTES)} limit). Use offset=${nextOffset} to continue.]`;
                }
                details = { truncation };
              } else if (userLimitedLines !== void 0 && startLine + userLimitedLines < allLines.length) {
                const remaining = allLines.length - (startLine + userLimitedLines);
                const nextOffset = startLine + userLimitedLines + 1;
                outputText = `${truncation.content}

[${remaining} more lines in file. Use offset=${nextOffset} to continue.]`;
              } else {
                outputText = truncation.content;
              }
              content = [{ type: "text", text: outputText }];
            }
            if (aborted)
              return;
            signal?.removeEventListener("abort", onAbort);
            resolve5({ content, details });
          } catch (error) {
            signal?.removeEventListener("abort", onAbort);
            if (!aborted)
              reject(error);
          }
        })();
      });
    },
    ...readRenderers
  };
}
function createReadTool(cwd, options) {
  return wrapToolDefinition(createReadToolDefinition(cwd, options));
}

// target/vendor-pi/dist/core/tools/write.js
import { mkdir as fsMkdir, writeFile as fsWriteFile2 } from "fs/promises";
import { dirname as dirname6 } from "path";

// target/vendor-pi/dist/core/tools/renderers/write.js
var WriteCallRenderComponent = class extends Text {
  cache;
  constructor() {
    super("", 0, 0);
  }
};
var WRITE_PARTIAL_FULL_HIGHLIGHT_LINES = 50;
function highlightSingleLine(line, lang) {
  const highlighted = (0, theme_exports.highlightCode)(line, lang);
  return highlighted[0] ?? "";
}
function refreshWriteHighlightPrefix(cache) {
  const prefixCount = Math.min(WRITE_PARTIAL_FULL_HIGHLIGHT_LINES, cache.normalizedLines.length);
  if (prefixCount === 0)
    return;
  const prefixSource = cache.normalizedLines.slice(0, prefixCount).join("\n");
  const prefixHighlighted = (0, theme_exports.highlightCode)(prefixSource, cache.lang);
  for (let i = 0; i < prefixCount; i++) {
    cache.highlightedLines[i] = prefixHighlighted[i] ?? highlightSingleLine(cache.normalizedLines[i] ?? "", cache.lang);
  }
}
function rebuildWriteHighlightCacheFull(rawPath, fileContent) {
  const lang = rawPath ? (0, theme_exports.getLanguageFromPath)(rawPath) : void 0;
  if (!lang)
    return void 0;
  const displayContent = normalizeDisplayText(fileContent);
  const normalized = replaceTabs(displayContent);
  return {
    rawPath,
    lang,
    rawContent: fileContent,
    normalizedLines: normalized.split("\n"),
    highlightedLines: (0, theme_exports.highlightCode)(normalized, lang)
  };
}
function updateWriteHighlightCacheIncremental(cache, rawPath, fileContent) {
  const lang = rawPath ? (0, theme_exports.getLanguageFromPath)(rawPath) : void 0;
  if (!lang)
    return void 0;
  if (!cache)
    return rebuildWriteHighlightCacheFull(rawPath, fileContent);
  if (cache.lang !== lang || cache.rawPath !== rawPath)
    return rebuildWriteHighlightCacheFull(rawPath, fileContent);
  if (!fileContent.startsWith(cache.rawContent))
    return rebuildWriteHighlightCacheFull(rawPath, fileContent);
  if (fileContent.length === cache.rawContent.length)
    return cache;
  const deltaRaw = fileContent.slice(cache.rawContent.length);
  const deltaDisplay = normalizeDisplayText(deltaRaw);
  const deltaNormalized = replaceTabs(deltaDisplay);
  cache.rawContent = fileContent;
  if (cache.normalizedLines.length === 0) {
    cache.normalizedLines.push("");
    cache.highlightedLines.push("");
  }
  const segments = deltaNormalized.split("\n");
  const lastIndex = cache.normalizedLines.length - 1;
  cache.normalizedLines[lastIndex] += segments[0];
  cache.highlightedLines[lastIndex] = highlightSingleLine(cache.normalizedLines[lastIndex], cache.lang);
  for (let i = 1; i < segments.length; i++) {
    cache.normalizedLines.push(segments[i]);
    cache.highlightedLines.push(highlightSingleLine(segments[i], cache.lang));
  }
  refreshWriteHighlightPrefix(cache);
  return cache;
}
function trimTrailingEmptyLines2(lines) {
  let end = lines.length;
  while (end > 0 && lines[end - 1] === "") {
    end--;
  }
  return lines.slice(0, end);
}
function formatWriteCall(args, options, theme31, cache, cwd) {
  const rawPath = str(args?.file_path ?? args?.path);
  const fileContent = str(args?.content);
  const pathDisplay = renderToolPath(rawPath, theme31, cwd);
  let text = `${theme31.fg("toolTitle", theme31.bold("write"))} ${pathDisplay}`;
  if (fileContent === null) {
    text += `

${theme31.fg("error", "[invalid content arg - expected string]")}`;
  } else if (fileContent) {
    const lang = rawPath ? (0, theme_exports.getLanguageFromPath)(rawPath) : void 0;
    const renderedLines = lang ? cache?.highlightedLines ?? (0, theme_exports.highlightCode)(replaceTabs(normalizeDisplayText(fileContent)), lang) : normalizeDisplayText(fileContent).split("\n");
    const lines = trimTrailingEmptyLines2(renderedLines);
    const totalLines = lines.length;
    const maxLines = options.expanded ? lines.length : 10;
    const displayLines = lines.slice(0, maxLines);
    const remaining = lines.length - maxLines;
    text += `

${displayLines.map((line) => lang ? line : theme31.fg("toolOutput", replaceTabs(line))).join("\n")}`;
    if (remaining > 0) {
      text += `${theme31.fg("muted", `
... (${remaining} more lines, ${totalLines} total,`)} ${keyHint("app.tools.expand", "to expand")}${theme31.fg("muted", ")")}`;
    }
  }
  return text;
}
function formatWriteResult(result, theme31) {
  if (!result.isError) {
    return void 0;
  }
  const output = result.content.filter((c) => c.type === "text").map((c) => c.text || "").join("\n");
  if (!output) {
    return void 0;
  }
  return `
${theme31.fg("error", output)}`;
}
var writeRenderers = {
  renderCall(args, theme31, context) {
    const renderArgs = args;
    const rawPath = str(renderArgs?.file_path ?? renderArgs?.path);
    const fileContent = str(renderArgs?.content);
    const component = context.lastComponent ?? new WriteCallRenderComponent();
    if (fileContent !== null) {
      component.cache = context.argsComplete ? rebuildWriteHighlightCacheFull(rawPath, fileContent) : updateWriteHighlightCacheIncremental(component.cache, rawPath, fileContent);
    } else {
      component.cache = void 0;
    }
    component.setText(formatWriteCall(renderArgs, { expanded: context.expanded, isPartial: context.isPartial }, theme31, component.cache, context.cwd));
    return component;
  },
  renderResult(result, _options, theme31, context) {
    const output = formatWriteResult({ ...result, isError: context.isError }, theme31);
    if (!output) {
      const component = context.lastComponent ?? new Container();
      component.clear();
      return component;
    }
    const text = context.lastComponent ?? new Text("", 0, 0);
    text.setText(output);
    return text;
  }
};

// target/vendor-pi/dist/core/tools/write.js
var writeSchema = typebox_exports.Object({
  path: typebox_exports.String({ description: "Path to the file to write (relative or absolute)" }),
  content: typebox_exports.String({ description: "Content to write to the file" })
});
var writeToolSystemPromptContribution = {
  snippet: "Create or overwrite files",
  guidelines: ["Use write only for new files or complete rewrites."]
};
var defaultWriteOperations = {
  writeFile: (path4, content) => fsWriteFile2(path4, content, "utf-8"),
  mkdir: (dir) => fsMkdir(dir, { recursive: true }).then(() => {
  })
};
function createWriteToolDefinition(cwd, options) {
  const ops = options?.operations ?? defaultWriteOperations;
  return {
    name: "write",
    label: "write",
    description: "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Automatically creates parent directories.",
    promptSnippet: writeToolSystemPromptContribution.snippet,
    promptGuidelines: [...writeToolSystemPromptContribution.guidelines],
    parameters: writeSchema,
    constrainedSampling: { type: "json_schema", strict: "prefer" },
    async execute(_toolCallId, { path: path4, content }, signal, _onUpdate, ctx) {
      const absolutePath = resolveToCwd(path4, ctx?.cwd || cwd);
      const dir = dirname6(absolutePath);
      return withFileMutationQueue(absolutePath, async () => {
        const throwIfAborted = () => {
          if (signal?.aborted)
            throw new Error("Operation aborted");
        };
        throwIfAborted();
        await ops.mkdir(dir);
        throwIfAborted();
        await ops.writeFile(absolutePath, content);
        throwIfAborted();
        return {
          content: [{ type: "text", text: `Successfully wrote to ${path4}` }],
          details: void 0
        };
      });
    },
    ...writeRenderers
  };
}
function createWriteTool(cwd, options) {
  return wrapToolDefinition(createWriteToolDefinition(cwd, options));
}

// target/vendor-pi/dist/core/tools/index.js
var allToolNames = /* @__PURE__ */ new Set([
  "read",
  "bash",
  "powershell",
  "edit",
  "write",
  "grep",
  "find",
  "ls"
]);
function createToolDefinition(toolName, cwd, options) {
  switch (toolName) {
    case "read":
      return createReadToolDefinition(cwd, options?.read);
    case "bash":
      return createBashToolDefinition(cwd, options?.bash);
    case "powershell":
      return createPowerShellToolDefinition(cwd, options?.powershell);
    case "edit":
      return createEditToolDefinition(cwd, options?.edit);
    case "write":
      return createWriteToolDefinition(cwd, options?.write);
    case "grep":
      return createGrepToolDefinition(cwd, options?.grep);
    case "find":
      return createFindToolDefinition(cwd, options?.find);
    case "ls":
      return createLsToolDefinition(cwd, options?.ls);
    default:
      throw new Error(`Unknown tool name: ${toolName}`);
  }
}
function createTool(toolName, cwd, options) {
  switch (toolName) {
    case "read":
      return createReadTool(cwd, options?.read);
    case "bash":
      return createBashTool(cwd, options?.bash);
    case "powershell":
      return createPowerShellTool(cwd, options?.powershell);
    case "edit":
      return createEditTool(cwd, options?.edit);
    case "write":
      return createWriteTool(cwd, options?.write);
    case "grep":
      return createGrepTool(cwd, options?.grep);
    case "find":
      return createFindTool(cwd, options?.find);
    case "ls":
      return createLsTool(cwd, options?.ls);
    default:
      throw new Error(`Unknown tool name: ${toolName}`);
  }
}
function createCodingToolDefinitions(cwd, options) {
  return [
    createReadToolDefinition(cwd, options?.read),
    createBashToolDefinition(cwd, options?.bash),
    createEditToolDefinition(cwd, options?.edit),
    createWriteToolDefinition(cwd, options?.write)
  ];
}
function createReadOnlyToolDefinitions(cwd, options) {
  return [
    createReadToolDefinition(cwd, options?.read),
    createGrepToolDefinition(cwd, options?.grep),
    createFindToolDefinition(cwd, options?.find),
    createLsToolDefinition(cwd, options?.ls)
  ];
}
function createAllToolDefinitions(cwd, options) {
  return {
    read: createReadToolDefinition(cwd, options?.read),
    bash: createBashToolDefinition(cwd, options?.bash),
    powershell: createPowerShellToolDefinition(cwd, options?.powershell),
    edit: createEditToolDefinition(cwd, options?.edit),
    write: createWriteToolDefinition(cwd, options?.write),
    grep: createGrepToolDefinition(cwd, options?.grep),
    find: createFindToolDefinition(cwd, options?.find),
    ls: createLsToolDefinition(cwd, options?.ls)
  };
}
function createCodingTools(cwd, options) {
  return [
    createReadTool(cwd, options?.read),
    createBashTool(cwd, options?.bash),
    createEditTool(cwd, options?.edit),
    createWriteTool(cwd, options?.write)
  ];
}
function createReadOnlyTools(cwd, options) {
  return [
    createReadTool(cwd, options?.read),
    createGrepTool(cwd, options?.grep),
    createFindTool(cwd, options?.find),
    createLsTool(cwd, options?.ls)
  ];
}
function createAllTools(cwd, options) {
  return {
    read: createReadTool(cwd, options?.read),
    bash: createBashTool(cwd, options?.bash),
    powershell: createPowerShellTool(cwd, options?.powershell),
    edit: createEditTool(cwd, options?.edit),
    write: createWriteTool(cwd, options?.write),
    grep: createGrepTool(cwd, options?.grep),
    find: createFindTool(cwd, options?.find),
    ls: createLsTool(cwd, options?.ls)
  };
}

// target/vendor-pi/dist/modes/interactive/components/armin.js
var WIDTH = 31;
var HEIGHT = 36;
var BITS = [
  255,
  255,
  255,
  127,
  255,
  240,
  255,
  127,
  255,
  237,
  255,
  127,
  255,
  219,
  255,
  127,
  255,
  183,
  255,
  127,
  255,
  119,
  254,
  127,
  63,
  248,
  254,
  127,
  223,
  255,
  254,
  127,
  223,
  63,
  252,
  127,
  159,
  195,
  251,
  127,
  111,
  252,
  244,
  127,
  247,
  15,
  247,
  127,
  247,
  255,
  247,
  127,
  247,
  255,
  227,
  127,
  247,
  7,
  232,
  127,
  239,
  248,
  103,
  112,
  15,
  255,
  187,
  111,
  241,
  0,
  208,
  91,
  253,
  63,
  236,
  83,
  193,
  255,
  239,
  87,
  159,
  253,
  238,
  95,
  159,
  252,
  174,
  95,
  31,
  120,
  172,
  95,
  63,
  0,
  80,
  108,
  127,
  0,
  220,
  119,
  255,
  192,
  63,
  120,
  255,
  1,
  248,
  127,
  255,
  3,
  156,
  120,
  255,
  7,
  140,
  124,
  255,
  15,
  206,
  120,
  255,
  255,
  207,
  127,
  255,
  255,
  207,
  120,
  255,
  255,
  223,
  120,
  255,
  255,
  223,
  125,
  255,
  255,
  63,
  126,
  255,
  255,
  255,
  127
];
var BYTES_PER_ROW = Math.ceil(WIDTH / 8);
var DISPLAY_HEIGHT = Math.ceil(HEIGHT / 2);
var EFFECTS = ["typewriter", "scanline", "rain", "fade", "crt", "glitch", "dissolve"];
function getPixel(x, y) {
  if (y >= HEIGHT)
    return false;
  const byteIndex = y * BYTES_PER_ROW + Math.floor(x / 8);
  const bitIndex = x % 8;
  return (BITS[byteIndex] >> bitIndex & 1) === 0;
}
function getChar(x, row) {
  const upper = getPixel(x, row * 2);
  const lower = getPixel(x, row * 2 + 1);
  if (upper && lower)
    return "\u2588";
  if (upper)
    return "\u2580";
  if (lower)
    return "\u2584";
  return " ";
}
function buildFinalGrid() {
  const grid = [];
  for (let row = 0; row < DISPLAY_HEIGHT; row++) {
    const line = [];
    for (let x = 0; x < WIDTH; x++) {
      line.push(getChar(x, row));
    }
    grid.push(line);
  }
  return grid;
}
var ArminComponent = class {
  ui;
  interval = null;
  effect;
  finalGrid;
  currentGrid;
  effectState = {};
  cachedLines = [];
  cachedWidth = 0;
  gridVersion = 0;
  cachedVersion = -1;
  constructor(ui) {
    this.ui = ui;
    this.effect = EFFECTS[Math.floor(Math.random() * EFFECTS.length)];
    this.finalGrid = buildFinalGrid();
    this.currentGrid = this.createEmptyGrid();
    this.initEffect();
    this.startAnimation();
  }
  invalidate() {
    this.cachedWidth = 0;
  }
  render(width) {
    if (width === this.cachedWidth && this.cachedVersion === this.gridVersion) {
      return this.cachedLines;
    }
    const padding = 1;
    const availableWidth = width - padding;
    this.cachedLines = this.currentGrid.map((row) => {
      const clipped = row.slice(0, availableWidth).join("");
      const padRight = Math.max(0, width - padding - clipped.length);
      return ` ${theme_exports.theme.fg("accent", clipped)}${" ".repeat(padRight)}`;
    });
    const message = "ARMIN SAYS HI";
    const msgPadRight = Math.max(0, width - padding - message.length);
    this.cachedLines.push(` ${theme_exports.theme.fg("accent", message)}${" ".repeat(msgPadRight)}`);
    this.cachedWidth = width;
    this.cachedVersion = this.gridVersion;
    return this.cachedLines;
  }
  createEmptyGrid() {
    return Array.from({ length: DISPLAY_HEIGHT }, () => Array(WIDTH).fill(" "));
  }
  initEffect() {
    switch (this.effect) {
      case "typewriter":
        this.effectState = { pos: 0 };
        break;
      case "scanline":
        this.effectState = { row: 0 };
        break;
      case "rain":
        this.effectState = {
          drops: Array.from({ length: WIDTH }, () => ({
            y: -Math.floor(Math.random() * DISPLAY_HEIGHT * 2),
            settled: 0
          }))
        };
        break;
      case "fade": {
        const positions = [];
        for (let row = 0; row < DISPLAY_HEIGHT; row++) {
          for (let x = 0; x < WIDTH; x++) {
            positions.push([row, x]);
          }
        }
        for (let i = positions.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [positions[i], positions[j]] = [positions[j], positions[i]];
        }
        this.effectState = { positions, idx: 0 };
        break;
      }
      case "crt":
        this.effectState = { expansion: 0 };
        break;
      case "glitch":
        this.effectState = { phase: 0, glitchFrames: 8 };
        break;
      case "dissolve": {
        this.currentGrid = Array.from({ length: DISPLAY_HEIGHT }, () => Array.from({ length: WIDTH }, () => {
          const chars = [" ", "\u2591", "\u2592", "\u2593", "\u2588", "\u2580", "\u2584"];
          return chars[Math.floor(Math.random() * chars.length)];
        }));
        const dissolvePositions = [];
        for (let row = 0; row < DISPLAY_HEIGHT; row++) {
          for (let x = 0; x < WIDTH; x++) {
            dissolvePositions.push([row, x]);
          }
        }
        for (let i = dissolvePositions.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [dissolvePositions[i], dissolvePositions[j]] = [dissolvePositions[j], dissolvePositions[i]];
        }
        this.effectState = { positions: dissolvePositions, idx: 0 };
        break;
      }
    }
  }
  startAnimation() {
    const fps = this.effect === "glitch" ? 60 : 30;
    this.interval = setInterval(() => {
      const done = this.tickEffect();
      this.updateDisplay();
      this.ui.requestRender();
      if (done) {
        this.stopAnimation();
      }
    }, 1e3 / fps);
  }
  stopAnimation() {
    if (this.interval) {
      clearInterval(this.interval);
      this.interval = null;
    }
  }
  tickEffect() {
    switch (this.effect) {
      case "typewriter":
        return this.tickTypewriter();
      case "scanline":
        return this.tickScanline();
      case "rain":
        return this.tickRain();
      case "fade":
        return this.tickFade();
      case "crt":
        return this.tickCrt();
      case "glitch":
        return this.tickGlitch();
      case "dissolve":
        return this.tickDissolve();
      default:
        return true;
    }
  }
  tickTypewriter() {
    const state = this.effectState;
    const pixelsPerFrame = 3;
    for (let i = 0; i < pixelsPerFrame; i++) {
      const row = Math.floor(state.pos / WIDTH);
      const x = state.pos % WIDTH;
      if (row >= DISPLAY_HEIGHT)
        return true;
      this.currentGrid[row][x] = this.finalGrid[row][x];
      state.pos++;
    }
    return false;
  }
  tickScanline() {
    const state = this.effectState;
    if (state.row >= DISPLAY_HEIGHT)
      return true;
    for (let x = 0; x < WIDTH; x++) {
      this.currentGrid[state.row][x] = this.finalGrid[state.row][x];
    }
    state.row++;
    return false;
  }
  tickRain() {
    const state = this.effectState;
    let allSettled = true;
    this.currentGrid = this.createEmptyGrid();
    for (let x = 0; x < WIDTH; x++) {
      const drop = state.drops[x];
      for (let row = DISPLAY_HEIGHT - 1; row >= DISPLAY_HEIGHT - drop.settled; row--) {
        if (row >= 0) {
          this.currentGrid[row][x] = this.finalGrid[row][x];
        }
      }
      if (drop.settled >= DISPLAY_HEIGHT)
        continue;
      allSettled = false;
      let targetRow = -1;
      for (let row = DISPLAY_HEIGHT - 1 - drop.settled; row >= 0; row--) {
        if (this.finalGrid[row][x] !== " ") {
          targetRow = row;
          break;
        }
      }
      drop.y++;
      if (drop.y >= 0 && drop.y < DISPLAY_HEIGHT) {
        if (targetRow >= 0 && drop.y >= targetRow) {
          drop.settled = DISPLAY_HEIGHT - targetRow;
          drop.y = -Math.floor(Math.random() * 5) - 1;
        } else {
          this.currentGrid[drop.y][x] = "\u2593";
        }
      }
    }
    return allSettled;
  }
  tickFade() {
    const state = this.effectState;
    const pixelsPerFrame = 15;
    for (let i = 0; i < pixelsPerFrame; i++) {
      if (state.idx >= state.positions.length)
        return true;
      const [row, x] = state.positions[state.idx];
      this.currentGrid[row][x] = this.finalGrid[row][x];
      state.idx++;
    }
    return false;
  }
  tickCrt() {
    const state = this.effectState;
    const midRow = Math.floor(DISPLAY_HEIGHT / 2);
    this.currentGrid = this.createEmptyGrid();
    const top = midRow - state.expansion;
    const bottom = midRow + state.expansion;
    for (let row = Math.max(0, top); row <= Math.min(DISPLAY_HEIGHT - 1, bottom); row++) {
      for (let x = 0; x < WIDTH; x++) {
        this.currentGrid[row][x] = this.finalGrid[row][x];
      }
    }
    state.expansion++;
    return state.expansion > DISPLAY_HEIGHT;
  }
  tickGlitch() {
    const state = this.effectState;
    if (state.phase < state.glitchFrames) {
      this.currentGrid = this.finalGrid.map((row) => {
        const offset = Math.floor(Math.random() * 7) - 3;
        const glitchRow = [...row];
        if (Math.random() < 0.3) {
          const shifted = glitchRow.slice(offset).concat(glitchRow.slice(0, offset));
          return shifted.slice(0, WIDTH);
        }
        if (Math.random() < 0.2) {
          const swapRow = Math.floor(Math.random() * DISPLAY_HEIGHT);
          return [...this.finalGrid[swapRow]];
        }
        return glitchRow;
      });
      state.phase++;
      return false;
    }
    this.currentGrid = this.finalGrid.map((row) => [...row]);
    return true;
  }
  tickDissolve() {
    const state = this.effectState;
    const pixelsPerFrame = 20;
    for (let i = 0; i < pixelsPerFrame; i++) {
      if (state.idx >= state.positions.length)
        return true;
      const [row, x] = state.positions[state.idx];
      this.currentGrid[row][x] = this.finalGrid[row][x];
      state.idx++;
    }
    return false;
  }
  updateDisplay() {
    this.gridVersion++;
  }
  dispose() {
    this.stopAnimation();
  }
};

// target/vendor-pi/dist/modes/interactive/components/markdown-transform.js
function createMarkdownTransform(messageType, isStreaming, transformers) {
  return (markdown, availableWidth) => applyMarkdownTransformers(markdown, { messageType, isStreaming, availableWidth }, transformers);
}
function applyMarkdownTransformers(markdown, context, transformers) {
  let transformedMarkdown = markdown;
  for (const transformer of transformers) {
    try {
      const transformed = transformer(transformedMarkdown, context);
      if (typeof transformed === "string") {
        transformedMarkdown = transformed;
      }
    } catch {
    }
  }
  return transformedMarkdown;
}

// target/vendor-pi/dist/modes/interactive/components/assistant-message.js
var OSC133_ZONE_START = "\x1B]133;A\x07";
var OSC133_ZONE_END = "\x1B]133;B\x07";
var OSC133_ZONE_FINAL = "\x1B]133;C\x07";
var AssistantMessageComponent = class extends Container {
  contentContainer;
  hideThinkingBlock;
  markdownTheme;
  hiddenThinkingLabel;
  outputPad;
  markdownTransformers;
  lastMessage;
  hasToolCalls = false;
  isStreaming = false;
  thinkingVisibilityOverrides = /* @__PURE__ */ new Map();
  constructor(message, hideThinkingBlock = false, markdownTheme = (0, theme_exports.getMarkdownTheme)(), hiddenThinkingLabel = "Thinking...", outputPad = 1, markdownTransformers = []) {
    super();
    this.hideThinkingBlock = hideThinkingBlock;
    this.markdownTheme = markdownTheme;
    this.hiddenThinkingLabel = hiddenThinkingLabel;
    this.outputPad = outputPad;
    this.markdownTransformers = markdownTransformers;
    this.contentContainer = new Container();
    this.addChild(this.contentContainer);
    if (message) {
      this.updateContent(message);
    }
  }
  invalidate() {
    super.invalidate();
    if (this.lastMessage) {
      this.updateContent(this.lastMessage);
    }
  }
  setHideThinkingBlock(hide) {
    this.hideThinkingBlock = hide;
    this.thinkingVisibilityOverrides.clear();
    if (this.lastMessage) {
      this.updateContent(this.lastMessage);
    }
  }
  setHiddenThinkingLabel(label) {
    this.hiddenThinkingLabel = label;
    if (this.lastMessage) {
      this.updateContent(this.lastMessage);
    }
  }
  setOutputPad(padding) {
    this.outputPad = padding;
    if (this.lastMessage) {
      this.updateContent(this.lastMessage);
    }
  }
  render(width) {
    const lines = super.render(width);
    if (this.hasToolCalls || lines.length === 0) {
      return lines;
    }
    lines[0] = OSC133_ZONE_START + lines[0];
    lines[lines.length - 1] = OSC133_ZONE_END + OSC133_ZONE_FINAL + lines[lines.length - 1];
    return lines;
  }
  updateContent(message, isStreaming = this.isStreaming) {
    this.lastMessage = message;
    this.isStreaming = isStreaming;
    this.contentContainer.clear();
    const hasVisibleContent = message.content.some((c) => c.type === "text" && c.text.trim() || c.type === "thinking" && c.thinking.trim());
    if (hasVisibleContent) {
      this.contentContainer.addChild(new Spacer(1));
    }
    let thinkingRunIndex = 0;
    for (let i = 0; i < message.content.length; i++) {
      const content = message.content[i];
      if (content.type === "text" && content.text.trim()) {
        this.contentContainer.addChild(new Markdown(content.text.trim(), this.outputPad, 0, this.markdownTheme, void 0, {
          transform: createMarkdownTransform("assistant", this.isStreaming, this.markdownTransformers)
        }));
      } else if (content.type === "thinking") {
        const thinkingBlocks = [];
        for (; i < message.content.length; i++) {
          const thinkingContent = message.content[i];
          if (thinkingContent.type !== "thinking") {
            break;
          }
          const thinking = thinkingContent.thinking.trim();
          if (thinking) {
            thinkingBlocks.push(thinking);
          }
        }
        i--;
        if (thinkingBlocks.length === 0) {
          continue;
        }
        const hasVisibleContentAfter = message.content.slice(i + 1).some((c) => c.type === "text" && c.text.trim() || c.type === "thinking" && c.thinking.trim());
        const runIndex = thinkingRunIndex++;
        const hidden = this.thinkingVisibilityOverrides.get(runIndex) ?? this.hideThinkingBlock;
        const thinkingComponent = hidden ? new Text(theme_exports.theme.italic(theme_exports.theme.fg("thinkingText", this.hiddenThinkingLabel)), this.outputPad, 0) : new Markdown(thinkingBlocks.join("\n\n"), this.outputPad, 0, this.markdownTheme, {
          color: (text) => theme_exports.theme.fg("thinkingText", text),
          italic: true
        }, {
          transform: createMarkdownTransform("assistant-thinking", this.isStreaming, this.markdownTransformers)
        });
        this.contentContainer.addChild(new MouseRegion(thinkingComponent, (event) => {
          if (event.type !== "click" || event.button !== "left")
            return void 0;
          this.thinkingVisibilityOverrides.set(runIndex, !hidden);
          if (this.lastMessage)
            this.updateContent(this.lastMessage);
          return { handled: true };
        }));
        if (hasVisibleContentAfter) {
          this.contentContainer.addChild(new Spacer(1));
        }
      }
    }
    const hasToolCalls = message.content.some((c) => c.type === "toolCall");
    this.hasToolCalls = hasToolCalls;
    if (message.stopReason === "length") {
      this.contentContainer.addChild(new Spacer(1));
      this.contentContainer.addChild(new Text(theme_exports.theme.fg("error", "Response was truncated before completion."), this.outputPad, 0));
    } else if (!hasToolCalls) {
      if (message.stopReason === "aborted") {
        const abortMessage = message.errorMessage && message.errorMessage !== "Request was aborted" ? message.errorMessage : "Operation aborted";
        this.contentContainer.addChild(new Spacer(1));
        this.contentContainer.addChild(new Text(theme_exports.theme.fg("error", abortMessage), this.outputPad, 0));
      } else if (message.stopReason === "error") {
        const errorMsg = message.errorMessage || "Unknown error";
        this.contentContainer.addChild(new Spacer(1));
        this.contentContainer.addChild(new Text(theme_exports.theme.fg("error", `Error: ${errorMsg}`), this.outputPad, 0));
      }
    }
  }
};

// target/vendor-pi/dist/modes/interactive/components/dynamic-border.js
var DynamicBorder = class {
  color;
  constructor(color = (str2) => theme_exports.theme.fg("border", str2)) {
    this.color = color;
  }
  invalidate() {
  }
  render(width) {
    return [this.color("\u2500".repeat(Math.max(1, width)))];
  }
};

// target/vendor-pi/dist/modes/interactive/components/bash-execution.js
var PREVIEW_LINES = 20;
var BashExecutionComponent = class extends Container {
  command;
  outputLines = [];
  status = "running";
  exitCode = void 0;
  loader;
  truncationResult;
  fullOutputPath;
  expanded = false;
  contentContainer;
  constructor(command, ui, excludeFromContext = false) {
    super();
    this.command = command;
    const colorKey = excludeFromContext ? "dim" : "bashMode";
    const borderColor = (str2) => theme_exports.theme.fg(colorKey, str2);
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder(borderColor));
    this.contentContainer = new Container();
    this.addChild(this.contentContainer);
    const header = new Text(theme_exports.theme.fg(colorKey, theme_exports.theme.bold(`$ ${command}`)), 1, 0);
    this.contentContainer.addChild(header);
    this.loader = new Loader(ui, (spinner) => theme_exports.theme.fg(colorKey, spinner), (text) => theme_exports.theme.fg("muted", text), `Running... (${keyText("tui.select.cancel")} to cancel)`);
    this.contentContainer.addChild(this.loader);
    this.addChild(new DynamicBorder(borderColor));
  }
  /**
   * Set whether the output is expanded (shows full output) or collapsed (preview only).
   */
  setExpanded(expanded) {
    this.expanded = expanded;
    this.updateDisplay();
  }
  invalidate() {
    super.invalidate();
    this.updateDisplay();
  }
  appendOutput(chunk) {
    const clean = stripAnsi(chunk).replace(/\r\n/g, "\n").replace(/\r/g, "\n");
    const newLines = clean.split("\n");
    if (this.outputLines.length > 0 && newLines.length > 0) {
      this.outputLines[this.outputLines.length - 1] += newLines[0];
      this.outputLines.push(...newLines.slice(1));
    } else {
      this.outputLines.push(...newLines);
    }
    this.updateDisplay();
  }
  setComplete(exitCode, cancelled, truncationResult, fullOutputPath) {
    this.exitCode = exitCode;
    this.status = cancelled ? "cancelled" : exitCode !== 0 && exitCode !== void 0 && exitCode !== null ? "error" : "complete";
    this.truncationResult = truncationResult;
    this.fullOutputPath = fullOutputPath;
    this.loader.stop();
    this.updateDisplay();
  }
  updateDisplay() {
    const fullOutput = this.outputLines.join("\n");
    const contextTruncation = truncateTail(fullOutput, {
      maxLines: DEFAULT_MAX_LINES,
      maxBytes: DEFAULT_MAX_BYTES
    });
    const availableLines = contextTruncation.content ? contextTruncation.content.split("\n") : [];
    const previewLogicalLines = availableLines.slice(-PREVIEW_LINES);
    const hiddenLineCount = availableLines.length - previewLogicalLines.length;
    this.contentContainer.clear();
    const header = new Text(theme_exports.theme.fg("bashMode", theme_exports.theme.bold(`$ ${this.command}`)), 1, 0);
    this.contentContainer.addChild(header);
    if (availableLines.length > 0) {
      if (this.expanded) {
        const displayText = availableLines.map((line) => theme_exports.theme.fg("muted", line)).join("\n");
        this.contentContainer.addChild(new Text(`
${displayText}`, 1, 0));
      } else {
        const styledOutput = previewLogicalLines.map((line) => theme_exports.theme.fg("muted", line)).join("\n");
        const styledInput = `
${styledOutput}`;
        let cachedWidth;
        let cachedLines;
        this.contentContainer.addChild({
          render: (width) => {
            if (cachedLines === void 0 || cachedWidth !== width) {
              const result = truncateToVisualLines(styledInput, PREVIEW_LINES, width, 1);
              cachedLines = result.visualLines;
              cachedWidth = width;
            }
            return cachedLines ?? [];
          },
          invalidate: () => {
            cachedWidth = void 0;
            cachedLines = void 0;
          }
        });
      }
    }
    if (this.status === "running") {
      this.contentContainer.addChild(this.loader);
    } else {
      const statusParts = [];
      if (hiddenLineCount > 0) {
        if (this.expanded) {
          statusParts.push(`${theme_exports.theme.fg("muted", "(")}${keyHint("app.tools.expand", "to collapse")}${theme_exports.theme.fg("muted", ")")}`);
        } else {
          statusParts.push(`${theme_exports.theme.fg("muted", `... ${hiddenLineCount} more lines (`)}${keyHint("app.tools.expand", "to expand")}${theme_exports.theme.fg("muted", ")")}`);
        }
      }
      if (this.status === "cancelled") {
        statusParts.push(theme_exports.theme.fg("warning", "(cancelled)"));
      } else if (this.status === "error") {
        statusParts.push(theme_exports.theme.fg("error", `(exit ${this.exitCode})`));
      }
      const wasTruncated = this.truncationResult?.truncated || contextTruncation.truncated;
      if (wasTruncated && this.fullOutputPath) {
        statusParts.push(theme_exports.theme.fg("warning", `Output truncated. Full output: ${this.fullOutputPath}`));
      }
      if (statusParts.length > 0) {
        this.contentContainer.addChild(new Text(`
${statusParts.join("\n")}`, 1, 0));
      }
    }
  }
  /**
   * Get the raw output for creating BashExecutionMessage.
   */
  getOutput() {
    return this.outputLines.join("\n");
  }
  /**
   * Get the command that was executed.
   */
  getCommand() {
    return this.command;
  }
};

// target/vendor-pi/dist/modes/interactive/components/branch-summary-message.js
var BranchSummaryMessageComponent = class extends Box {
  expanded = false;
  message;
  markdownTheme;
  constructor(message, markdownTheme = (0, theme_exports.getMarkdownTheme)()) {
    super(1, 1, (t) => theme_exports.theme.bg("customMessageBg", t));
    this.message = message;
    this.markdownTheme = markdownTheme;
    this.updateDisplay();
  }
  setExpanded(expanded) {
    this.expanded = expanded;
    this.updateDisplay();
  }
  invalidate() {
    super.invalidate();
    this.updateDisplay();
  }
  updateDisplay() {
    this.clear();
    const content = new Container();
    const label = theme_exports.theme.fg("customMessageLabel", `\x1B[1m[branch]\x1B[22m`);
    content.addChild(new Text(label, 0, 0));
    content.addChild(new Spacer(1));
    if (this.expanded) {
      const header = "**Branch Summary**\n\n";
      content.addChild(new Markdown(header + this.message.summary, 0, 0, this.markdownTheme, {
        color: (text) => theme_exports.theme.fg("customMessageText", text)
      }));
    } else {
      content.addChild(new Text(theme_exports.theme.fg("customMessageText", "Branch summary (") + theme_exports.theme.fg("dim", keyText("app.tools.expand")) + theme_exports.theme.fg("customMessageText", " to expand)"), 0, 0));
    }
    this.addChild(new MouseRegion(content, (event) => {
      if (event.type !== "click" || event.button !== "left")
        return void 0;
      this.setExpanded(!this.expanded);
      return { handled: true };
    }));
  }
};

// target/vendor-pi/dist/modes/interactive/components/compaction-summary-message.js
var CompactionSummaryMessageComponent = class extends Box {
  expanded = false;
  message;
  markdownTheme;
  constructor(message, markdownTheme = (0, theme_exports.getMarkdownTheme)()) {
    super(1, 1, (t) => theme_exports.theme.bg("customMessageBg", t));
    this.message = message;
    this.markdownTheme = markdownTheme;
    this.updateDisplay();
  }
  setExpanded(expanded) {
    this.expanded = expanded;
    this.updateDisplay();
  }
  invalidate() {
    super.invalidate();
    this.updateDisplay();
  }
  updateDisplay() {
    this.clear();
    const content = new Container();
    const tokenStr = this.message.tokensBefore.toLocaleString();
    const label = theme_exports.theme.fg("customMessageLabel", `\x1B[1m[compaction]\x1B[22m`);
    content.addChild(new Text(label, 0, 0));
    content.addChild(new Spacer(1));
    if (this.expanded) {
      const header = `**Compacted from ${tokenStr} tokens**

`;
      content.addChild(new Markdown(header + this.message.summary, 0, 0, this.markdownTheme, {
        color: (text) => theme_exports.theme.fg("customMessageText", text)
      }));
    } else {
      content.addChild(new Text(theme_exports.theme.fg("customMessageText", `Compacted from ${tokenStr} tokens (`) + theme_exports.theme.fg("dim", keyText("app.tools.expand")) + theme_exports.theme.fg("customMessageText", " to expand)"), 0, 0));
    }
    this.addChild(new MouseRegion(content, (event) => {
      if (event.type !== "click" || event.button !== "left")
        return void 0;
      this.setExpanded(!this.expanded);
      return { handled: true };
    }));
  }
};

// target/vendor-pi/dist/modes/interactive/components/custom-message.js
var CustomMessageComponent = class extends Container {
  message;
  customRenderer;
  box;
  customComponent;
  markdownTheme;
  _expanded = false;
  outputPad;
  constructor(message, customRenderer, markdownTheme = (0, theme_exports.getMarkdownTheme)(), outputPad = 1) {
    super();
    this.message = message;
    this.customRenderer = customRenderer;
    this.markdownTheme = markdownTheme;
    this.outputPad = outputPad;
    this.addChild(new Spacer(1));
    this.box = new Box(1, 1, (t) => theme_exports.theme.bg("customMessageBg", t));
    this.rebuild();
  }
  setExpanded(expanded) {
    if (this._expanded !== expanded) {
      this._expanded = expanded;
      this.rebuild();
    }
  }
  setOutputPad(outputPad) {
    if (this.outputPad !== outputPad) {
      this.outputPad = outputPad;
      this.rebuild();
    }
  }
  invalidate() {
    super.invalidate();
    this.rebuild();
  }
  rebuild() {
    if (this.customComponent) {
      this.removeChild(this.customComponent);
      this.customComponent = void 0;
    }
    this.removeChild(this.box);
    if (this.customRenderer) {
      try {
        const component = this.customRenderer(this.message, { expanded: this._expanded, outputPad: this.outputPad }, theme_exports.theme);
        if (component) {
          this.customComponent = component;
          this.addChild(component);
          return;
        }
      } catch {
      }
    }
    this.addChild(this.box);
    this.box.clear();
    const label = theme_exports.theme.fg("customMessageLabel", `\x1B[1m[${this.message.customType}]\x1B[22m`);
    this.box.addChild(new Text(label, 0, 0));
    this.box.addChild(new Spacer(1));
    let text;
    if (typeof this.message.content === "string") {
      text = this.message.content;
    } else {
      text = this.message.content.filter((c) => c.type === "text").map((c) => c.text).join("\n");
    }
    this.box.addChild(new Markdown(text, 0, 0, this.markdownTheme, {
      color: (text2) => theme_exports.theme.fg("customMessageText", text2)
    }));
  }
};

// target/vendor-pi/dist/modes/interactive/external-editor.js
import { spawn as spawn5 } from "node:child_process";
import { mkdtempSync, readFileSync as readFileSync5, rmSync as rmSync2, writeFileSync as writeFileSync5 } from "node:fs";
import { tmpdir as tmpdir2 } from "node:os";
import { join as join10 } from "node:path";
async function editInExternalEditor(options) {
  const directory = mkdtempSync(join10(tmpdir2(), "pi-editor-"));
  const filePath = join10(directory, "prompt.md");
  try {
    writeFileSync5(filePath, options.content, "utf-8");
    const [editor, ...editorArgs] = options.command.split(" ");
    process.stdout.write(`Launching external editor: ${options.command}
Pi will resume when the editor exits.
`);
    const exitCode = await new Promise((resolve5) => {
      const child = spawn5(editor, [...editorArgs, filePath], {
        stdio: "inherit",
        shell: process.platform === "win32"
      });
      child.on("error", () => resolve5(null));
      child.on("close", (code) => resolve5(code));
    });
    if (exitCode !== 0) {
      return { status: "failed" };
    }
    return { status: "complete", content: stripBom(readFileSync5(filePath, "utf-8")).replace(/\n$/, "") };
  } finally {
    try {
      rmSync2(directory, { recursive: true, force: true });
    } catch {
    }
  }
}

// target/vendor-pi/dist/modes/interactive/components/extension-editor.js
var ExtensionEditorComponent = class extends Container {
  editor;
  onSubmitCallback;
  onCancelCallback;
  tui;
  keybindings;
  externalEditorCommand;
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    this.editor.focused = value;
  }
  constructor(tui, keybindings, title, prefill, onSubmit, onCancel, options, externalEditorCommand) {
    super();
    this.tui = tui;
    this.keybindings = keybindings;
    this.externalEditorCommand = externalEditorCommand || process.env.VISUAL || process.env.EDITOR || (process.platform === "win32" ? "notepad" : "nano");
    this.onSubmitCallback = onSubmit;
    this.onCancelCallback = onCancel;
    const { description, ...editorOptions } = options ?? {};
    this.addChild(new DynamicBorder());
    this.addChild(new Spacer(1));
    this.addChild(new Text(theme_exports.theme.fg("accent", title), 1, 0));
    if (description) {
      this.addChild(new Spacer(1));
      this.addChild(new Text(theme_exports.theme.fg("text", description), 1, 0));
    }
    this.addChild(new Spacer(1));
    this.editor = new Editor(tui, (0, theme_exports.getEditorTheme)(), editorOptions);
    if (prefill) {
      this.editor.setText(prefill);
    }
    this.editor.onSubmit = (text) => {
      this.onSubmitCallback(text);
    };
    this.addChild(this.editor);
    this.addChild(new Spacer(1));
    const hint = keyHint("tui.select.confirm", "submit") + "  " + keyHint("tui.input.newLine", "newline") + "  " + keyHint("tui.select.cancel", "cancel") + `  ${keyHint("app.editor.external", "external editor")}`;
    this.addChild(new Text(hint, 1, 0));
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder());
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (kb.matches(keyData, "tui.select.cancel")) {
      this.onCancelCallback();
      return;
    }
    if (this.keybindings.matches(keyData, "app.editor.external")) {
      void this.handleOpenExternalEditor();
      return;
    }
    this.editor.handleInput(keyData);
  }
  async handleOpenExternalEditor() {
    const content = this.editor.getText();
    this.tui.stop();
    try {
      const result = await editInExternalEditor({
        command: this.externalEditorCommand,
        content
      });
      if (result.status === "complete") {
        this.editor.setText(result.content);
      }
    } finally {
      this.tui.start();
      this.tui.requestRender(true);
    }
  }
};

// target/vendor-pi/dist/modes/interactive/components/countdown-timer.js
var CountdownTimer = class {
  intervalId;
  remainingSeconds;
  tui;
  onTick;
  onExpire;
  constructor(timeoutMs, tui, onTick, onExpire) {
    this.tui = tui;
    this.onTick = onTick;
    this.onExpire = onExpire;
    this.remainingSeconds = Math.ceil(timeoutMs / 1e3);
    this.onTick(this.remainingSeconds);
    this.intervalId = setInterval(() => {
      this.remainingSeconds--;
      this.onTick(this.remainingSeconds);
      this.tui?.requestRender();
      if (this.remainingSeconds <= 0) {
        this.dispose();
        this.onExpire();
      }
    }, 1e3);
  }
  dispose() {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = void 0;
    }
  }
};

// target/vendor-pi/dist/modes/interactive/components/extension-input.js
var ExtensionInputComponent = class extends Container {
  input;
  onSubmitCallback;
  onCancelCallback;
  titleText;
  baseTitle;
  countdown;
  // Focusable implementation - propagate to input for IME cursor positioning
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    this.input.focused = value;
  }
  constructor(title, _placeholder, onSubmit, onCancel, opts) {
    super();
    this.onSubmitCallback = onSubmit;
    this.onCancelCallback = onCancel;
    this.baseTitle = title;
    this.addChild(new DynamicBorder());
    this.addChild(new Spacer(1));
    this.titleText = new Text(theme_exports.theme.fg("accent", title), 1, 0);
    this.addChild(this.titleText);
    if (opts?.description) {
      this.addChild(new Spacer(1));
      this.addChild(new Text(theme_exports.theme.fg("text", opts.description), 1, 0));
    }
    this.addChild(new Spacer(1));
    if (opts?.timeout && opts.timeout > 0 && opts.tui) {
      this.countdown = new CountdownTimer(opts.timeout, opts.tui, (s) => this.titleText.setText(theme_exports.theme.fg("accent", `${this.baseTitle} (${s}s)`)), () => this.onCancelCallback());
    }
    this.input = new Input();
    if (opts?.initialValue)
      this.input.setValue(opts.initialValue);
    this.addChild(this.input);
    this.addChild(new Spacer(1));
    this.addChild(new Text(`${keyHint("tui.select.confirm", "submit")}  ${keyHint("tui.select.cancel", "cancel")}`, 1, 0));
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder());
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (kb.matches(keyData, "tui.select.confirm") || keyData === "\n") {
      this.onSubmitCallback(this.input.getValue());
    } else if (kb.matches(keyData, "tui.select.cancel")) {
      this.onCancelCallback();
    } else {
      this.input.handleInput(keyData);
    }
  }
  dispose() {
    this.countdown?.dispose();
  }
};

// target/vendor-pi/dist/modes/interactive/components/extension-selector.js
var ExtensionSelectorComponent = class extends Container {
  options;
  selectedIndex = 0;
  listContainer;
  onSelectCallback;
  onCancelCallback;
  titleText;
  baseTitle;
  countdown;
  onToggleToolsExpanded;
  constructor(title, options, onSelect, onCancel, opts) {
    super();
    this.options = options;
    this.onSelectCallback = onSelect;
    this.onCancelCallback = onCancel;
    this.onToggleToolsExpanded = opts?.onToggleToolsExpanded;
    this.baseTitle = title;
    this.addChild(new DynamicBorder());
    this.addChild(new Spacer(1));
    this.titleText = new Text(theme_exports.theme.fg("accent", theme_exports.theme.bold(title)), 1, 0);
    this.addChild(this.titleText);
    if (opts?.description) {
      this.addChild(new Spacer(1));
      this.addChild(new Text(theme_exports.theme.fg("text", opts.description), 1, 0));
    }
    this.addChild(new Spacer(1));
    if (opts?.timeout && opts.timeout > 0 && opts.tui) {
      this.countdown = new CountdownTimer(opts.timeout, opts.tui, (s) => this.titleText.setText(theme_exports.theme.fg("accent", theme_exports.theme.bold(`${this.baseTitle} (${s}s)`))), () => this.onCancelCallback());
    }
    this.listContainer = new Container();
    this.addChild(this.listContainer);
    this.addChild(new Spacer(1));
    this.addChild(new Text(rawKeyHint("\u2191\u2193", "navigate") + "  " + keyHint("tui.select.confirm", "select") + "  " + keyHint("tui.select.cancel", "cancel"), 1, 0));
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder());
    this.updateList();
  }
  updateList() {
    this.listContainer.clear();
    for (let i = 0; i < this.options.length; i++) {
      const isSelected = i === this.selectedIndex;
      const text = isSelected ? theme_exports.theme.fg("accent", "\u2192 ") + theme_exports.theme.fg("accent", this.options[i]) : `  ${theme_exports.theme.fg("text", this.options[i])}`;
      this.listContainer.addChild(new Text(text, 1, 0));
    }
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (kb.matches(keyData, "app.tools.expand")) {
      this.onToggleToolsExpanded?.();
    } else if (kb.matches(keyData, "tui.select.up") || keyData === "k") {
      this.selectedIndex = Math.max(0, this.selectedIndex - 1);
      this.updateList();
    } else if (kb.matches(keyData, "tui.select.down") || keyData === "j") {
      this.selectedIndex = Math.min(this.options.length - 1, this.selectedIndex + 1);
      this.updateList();
    } else if (kb.matches(keyData, "tui.select.confirm") || keyData === "\n") {
      const selected = this.options[this.selectedIndex];
      if (selected)
        this.onSelectCallback(selected);
    } else if (kb.matches(keyData, "tui.select.cancel")) {
      this.onCancelCallback();
    }
  }
  dispose() {
    this.countdown?.dispose();
  }
};

// target/vendor-pi/dist/modes/interactive/theme/system-theme.js
var TOOL_PANELS = ["toolPendingBg", "toolSuccessBg", "toolErrorBg"];
var MESSAGE_PANELS = ["userMessageBg", "customMessageBg"];
var PANELS = [
  "userMessageBg",
  "toolPendingBg",
  "toolSuccessBg",
  "toolErrorBg",
  "selectedBg",
  "searchMatchBg",
  "customMessageBg"
];
var THINKING = [
  "thinkingOff",
  "thinkingMinimal",
  "thinkingLow",
  "thinkingMedium",
  "thinkingHigh",
  "thinkingXhigh",
  "thinkingMax"
];
var THINKING_LEVELS = [
  "thinking0",
  "thinking1",
  "thinking2",
  "thinking3",
  "thinking4",
  "thinking5",
  "thinking6"
];
var each = (tokens, on, level) => tokens.map((token) => ({ token, on, level }));
var RULES = [
  ...each(PANELS, ["background"], "panel"),
  { token: "text", on: ["background"], level: "text" },
  { token: "text", on: ["selectedBg"], level: "textOnPanel" },
  { token: "userMessageText", on: ["userMessageBg"], level: "textOnPanel" },
  { token: "toolTitle", on: TOOL_PANELS, level: "textOnPanel" },
  ...each(["accent", "success", "error", "warning"], ["background", "selectedBg", ...TOOL_PANELS], "readable"),
  { token: "muted", on: ["background", "selectedBg", "customMessageBg", ...TOOL_PANELS], level: "readable" },
  { token: "dim", on: ["background", "selectedBg", "customMessageBg", ...TOOL_PANELS], level: "subtle" },
  { token: "thinkingText", on: ["background"], level: "readable" },
  { token: "customMessageText", on: ["customMessageBg", ...TOOL_PANELS], level: "readable" },
  {
    token: "customMessageLabel",
    on: ["background", "customMessageBg", "selectedBg", ...TOOL_PANELS],
    level: "readable"
  },
  { token: "toolOutput", on: ["background", ...TOOL_PANELS], level: "readable" },
  ...each(["mdHeading", "mdLink", "mdLinkUrl", "mdCode", "mdQuote", "mdCodeBlockBorder", "mdListBullet"], ["background", ...MESSAGE_PANELS], "readable"),
  { token: "mdCodeBlock", on: ["background", ...MESSAGE_PANELS, ...TOOL_PANELS], level: "readable" },
  ...each(["toolDiffAdded", "toolDiffRemoved", "toolDiffContext"], ["background", ...TOOL_PANELS], "readable"),
  ...each([
    "syntaxComment",
    "syntaxKeyword",
    "syntaxFunction",
    "syntaxVariable",
    "syntaxString",
    "syntaxNumber",
    "syntaxType",
    "syntaxOperator",
    "syntaxPunctuation"
  ], ["background", ...MESSAGE_PANELS, ...TOOL_PANELS], "readable"),
  { token: "searchMatchText", on: ["searchMatchBg"], level: "readable" },
  ...each(["bashMode", "border", "borderAccent"], ["background"], "readable"),
  { token: "borderMuted", on: ["background"], level: "subtle" },
  ...each(["mdQuoteBorder", "mdHr"], ["background", ...MESSAGE_PANELS, ...TOOL_PANELS], "readable"),
  { token: "scrollbarTrack", on: ["background"], level: "track" },
  { token: "scrollbarThumb", on: ["scrollbarTrack"], level: "thumb" },
  ...THINKING.map((token, index) => ({ token, on: ["background"], level: THINKING_LEVELS[index] }))
];
var SOLVE_ORDER = (() => {
  const order = [];
  const visit = (token) => {
    if (order.includes(token))
      return;
    for (const rule of RULES) {
      if (rule.token !== token)
        continue;
      for (const surface of rule.on)
        if (surface !== "background")
          visit(surface);
    }
    order.push(token);
  };
  for (const rule of RULES)
    visit(rule.token);
  return order;
})();

// target/vendor-pi/dist/modes/interactive/components/footer.js
import { isAbsolute as isAbsolute3, relative as relative4, resolve as resolve4, sep as sep5 } from "node:path";

// target/vendor-pi/dist/core/experimental.js
function areExperimentalFeaturesEnabled() {
  return process.env.PI_EXPERIMENTAL === "1";
}

// target/vendor-pi/dist/modes/interactive/components/footer.js
function sanitizeStatusText(text) {
  return text.replace(/[\r\n\t]/g, " ").replace(/ +/g, " ").trim();
}
function formatTokens(count) {
  if (count < 1e3)
    return count.toString();
  if (count < 1e4)
    return `${(count / 1e3).toFixed(1)}k`;
  if (count < 1e6)
    return `${Math.round(count / 1e3)}k`;
  if (count < 1e7)
    return `${(count / 1e6).toFixed(1)}M`;
  return `${Math.round(count / 1e6)}M`;
}
function formatCwdForFooter(cwd, home) {
  if (!home)
    return cwd;
  const resolvedCwd = resolve4(cwd);
  const resolvedHome = resolve4(home);
  const relativeToHome = relative4(resolvedHome, resolvedCwd);
  const isInsideHome = relativeToHome === "" || relativeToHome !== ".." && !relativeToHome.startsWith(`..${sep5}`) && !isAbsolute3(relativeToHome);
  if (!isInsideHome)
    return cwd;
  return relativeToHome === "" ? "~" : `~${sep5}${relativeToHome}`;
}
var FooterComponent = class {
  autoCompactEnabled = true;
  session;
  footerData;
  sessionStats;
  constructor(session, footerData) {
    this.session = session;
    this.footerData = footerData;
  }
  setSession(session) {
    this.session = session;
  }
  setAutoCompactEnabled(enabled) {
    this.autoCompactEnabled = enabled;
  }
  /**
   * No-op: git branch caching now handled by provider.
   * Kept for compatibility with existing call sites in interactive-mode.
   */
  invalidate() {
  }
  /**
   * Clean up resources.
   * Git watcher cleanup now handled by provider.
   */
  dispose() {
  }
  /**
   * Usage totals and context usage scan the whole session, and the footer renders on every frame.
   * Entries are append-only and every append moves the leaf, so the results only change with the
   * session, leaf, entry count, or the model whose context window applies.
   */
  getSessionStats() {
    const sessionManager = this.session.sessionManager;
    const entryCount = sessionManager.getEntryCount();
    const sessionId = sessionManager.getSessionId();
    const leafId = sessionManager.getLeafId();
    const limitsModel = this.session.routedModel?.model ?? this.session.model;
    const cached = this.sessionStats;
    if (cached && cached.session === this.session && cached.sessionId === sessionId && cached.leafId === leafId && cached.entryCount === entryCount && cached.limitsModel === limitsModel) {
      return cached;
    }
    const usageTotals = createUsageTotals();
    let latestCacheHitRate;
    for (const entry of sessionManager.getEntries()) {
      if (entry.type === "usage") {
        addUsageToTotals(usageTotals, entry.usage);
      } else if (entry.type === "message" && entry.message.role === "assistant") {
        addUsageToTotals(usageTotals, entry.message.usage);
        const latestPromptTokens = entry.message.usage.input + entry.message.usage.cacheRead + entry.message.usage.cacheWrite;
        latestCacheHitRate = latestPromptTokens > 0 ? entry.message.usage.cacheRead / latestPromptTokens * 100 : void 0;
      } else if (entry.type === "message" && entry.message.role === "toolResult" && entry.message.usage) {
        addUsageToTotals(usageTotals, entry.message.usage);
      } else if ((entry.type === "branch_summary" || entry.type === "compaction") && entry.usage) {
        addUsageToTotals(usageTotals, entry.usage);
      }
    }
    const contextUsage = this.session.getContextUsage();
    this.sessionStats = {
      session: this.session,
      sessionId,
      leafId,
      entryCount,
      limitsModel,
      usageTotals,
      latestCacheHitRate,
      contextUsage
    };
    return this.sessionStats;
  }
  render(width) {
    const state = this.session.state;
    const { usageTotals, latestCacheHitRate, contextUsage } = this.getSessionStats();
    const contextWindow = contextUsage?.contextWindow ?? state.model?.contextWindow ?? 0;
    const contextPercentValue = contextUsage?.percent ?? 0;
    const contextPercent = contextUsage?.percent !== null ? contextPercentValue.toFixed(1) : "?";
    let pwd = formatCwdForFooter(this.session.sessionManager.getCwd(), process.env.HOME || process.env.USERPROFILE);
    const branch = this.footerData.getGitBranch();
    if (branch) {
      pwd = `${pwd} (${branch})`;
    }
    const sessionName = this.session.sessionManager.getSessionName();
    if (sessionName) {
      pwd = `${pwd} \u2022 ${sessionName}`;
    }
    const statsParts = [];
    if (usageTotals.input)
      statsParts.push(`\u2191${formatTokens(usageTotals.input)}`);
    if (usageTotals.output)
      statsParts.push(`\u2193${formatTokens(usageTotals.output)}`);
    if (usageTotals.cacheRead)
      statsParts.push(`R${formatTokens(usageTotals.cacheRead)}`);
    if (usageTotals.cacheWrite)
      statsParts.push(`W${formatTokens(usageTotals.cacheWrite)}`);
    if ((usageTotals.cacheRead > 0 || usageTotals.cacheWrite > 0) && latestCacheHitRate !== void 0) {
      statsParts.push(`CH${latestCacheHitRate.toFixed(1)}%`);
    }
    const usingSubscription = state.model ? state.model.provider === "kimi-coding" || this.session.modelRuntime.isUsingSubscription(state.model.provider) : false;
    if (usageTotals.cost || usingSubscription) {
      const costStr = `$${usageTotals.cost.toFixed(3)}${usingSubscription ? " (sub)" : ""}`;
      statsParts.push(costStr);
    }
    let contextPercentStr;
    const autoIndicator = this.autoCompactEnabled ? " (auto)" : "";
    const contextPercentDisplay = contextPercent === "?" ? `?/${formatTokens(contextWindow)}${autoIndicator}` : `${contextPercent}%/${formatTokens(contextWindow)}${autoIndicator}`;
    if (contextPercentValue > 90) {
      contextPercentStr = theme_exports.theme.fg("error", contextPercentDisplay);
    } else if (contextPercentValue > 70) {
      contextPercentStr = theme_exports.theme.fg("warning", contextPercentDisplay);
    } else {
      contextPercentStr = contextPercentDisplay;
    }
    statsParts.push(contextPercentStr);
    if (areExperimentalFeaturesEnabled()) {
      statsParts.push(`${theme_exports.theme.fg("dim", "\u2022")} ${theme_exports.theme.bold(theme_exports.theme.fg("warning", "xp"))}`);
    }
    let statsLeft = statsParts.join(" ");
    const modelName = state.model?.id || "no-model";
    let statsLeftWidth = visibleWidth(statsLeft);
    if (statsLeftWidth > width) {
      statsLeft = truncateToWidth(statsLeft, width, "...");
      statsLeftWidth = visibleWidth(statsLeft);
    }
    const minPadding = 2;
    let rightSideWithoutProvider = modelName;
    if (state.model?.reasoning) {
      const thinkingLevel = state.thinkingLevel || "off";
      rightSideWithoutProvider = thinkingLevel === "off" ? `${modelName} \u2022 thinking off` : `${modelName} \u2022 ${thinkingLevel}`;
    }
    const routed = this.session.routedModel;
    if (routed) {
      const level = routed.thinkingLevel ? ` \u2022 ${routed.thinkingLevel}` : "";
      rightSideWithoutProvider += ` \u2192 ${routed.model.id}${level}`;
    }
    let rightSide = rightSideWithoutProvider;
    if (this.footerData.getAvailableProviderCount() > 1 && state.model) {
      rightSide = `(${state.model.provider}) ${rightSideWithoutProvider}`;
      if (statsLeftWidth + minPadding + visibleWidth(rightSide) > width) {
        rightSide = rightSideWithoutProvider;
      }
    }
    const rightSideWidth = visibleWidth(rightSide);
    const totalNeeded = statsLeftWidth + minPadding + rightSideWidth;
    let statsLine;
    if (totalNeeded <= width) {
      const padding = " ".repeat(width - statsLeftWidth - rightSideWidth);
      statsLine = statsLeft + padding + rightSide;
    } else {
      const availableForRight = width - statsLeftWidth - minPadding;
      if (availableForRight > 0) {
        const truncatedRight = truncateToWidth(rightSide, availableForRight, "");
        const truncatedRightWidth = visibleWidth(truncatedRight);
        const padding = " ".repeat(Math.max(0, width - statsLeftWidth - truncatedRightWidth));
        statsLine = statsLeft + padding + truncatedRight;
      } else {
        statsLine = statsLeft;
      }
    }
    const dimStatsLeft = theme_exports.theme.fg("dim", statsLeft);
    const remainder = statsLine.slice(statsLeft.length);
    const dimRemainder = theme_exports.theme.fg("dim", remainder);
    const pwdLine = truncateToWidth(theme_exports.theme.fg("dim", pwd), width, theme_exports.theme.fg("dim", "..."));
    const lines = [pwdLine, dimStatsLeft + dimRemainder];
    const extensionStatuses = this.footerData.getExtensionStatuses();
    if (extensionStatuses.size > 0) {
      const sortedStatuses = Array.from(extensionStatuses.entries()).sort(([a], [b]) => a.localeCompare(b)).map(([, text]) => sanitizeStatusText(text));
      const statusLine = sortedStatuses.join(" ");
      lines.push(truncateToWidth(statusLine, width, theme_exports.theme.fg("dim", "...")));
    }
    return lines;
  }
};

// target/vendor-pi/dist/utils/open-browser.js
import { spawn as spawn6 } from "node:child_process";
function openBrowser(target) {
  const [cmd, args] = process.platform === "darwin" ? ["open", [target]] : process.platform === "win32" ? ["rundll32", ["url.dll,FileProtocolHandler", target]] : ["xdg-open", [target]];
  spawn6(cmd, args, { stdio: "ignore", detached: true }).on("error", () => {
  }).unref();
}

// target/vendor-pi/dist/modes/interactive/components/login-dialog.js
var LoginDialogComponent = class extends Container {
  contentContainer;
  input;
  tui;
  abortController = new AbortController();
  inputResolver;
  inputRejecter;
  onComplete;
  // Focusable implementation - propagate to input for IME cursor positioning
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    this.input.focused = value;
  }
  constructor(tui, providerId, onComplete, providerNameOverride, titleOverride) {
    super();
    this.tui = tui;
    this.onComplete = onComplete;
    const providerName = providerNameOverride || providerId;
    const title = titleOverride ?? `Login to ${providerName}`;
    this.addChild(new DynamicBorder());
    this.addChild(new Text(theme_exports.theme.fg("accent", theme_exports.theme.bold(title)), 1, 0));
    this.contentContainer = new Container();
    this.addChild(this.contentContainer);
    this.input = new Input();
    this.input.onSubmit = () => {
      if (this.inputResolver) {
        const value = this.input.getValue();
        this.replaceInputWithSubmittedText(value);
        this.inputResolver(value);
        this.inputResolver = void 0;
        this.inputRejecter = void 0;
      }
    };
    this.input.onEscape = () => {
      this.cancel();
    };
    this.addChild(new DynamicBorder());
  }
  get signal() {
    return this.abortController.signal;
  }
  replaceInputWithSubmittedText(value) {
    this.contentContainer.children = this.contentContainer.children.map((child) => child === this.input ? new Text(`> ${value}`, 0, 0) : child);
  }
  cancel() {
    this.abortController.abort();
    if (this.inputRejecter) {
      this.inputRejecter(new Error("Login cancelled"));
      this.inputResolver = void 0;
      this.inputRejecter = void 0;
    }
    this.onComplete(false, "Login cancelled");
  }
  /**
   * Called by onAuth callback - show URL and optional instructions
   */
  showAuth(url, instructions) {
    this.contentContainer.clear();
    this.contentContainer.addChild(new Spacer(1));
    const linkedUrl = `\x1B]8;;${url}\x07${url}\x1B]8;;\x07`;
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("accent", linkedUrl), 1, 0));
    const clickHint = process.platform === "darwin" ? "Cmd+click to open" : "Ctrl+click to open";
    const hyperlink2 = `\x1B]8;;${url}\x07${clickHint}\x1B]8;;\x07`;
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("dim", hyperlink2), 1, 0));
    if (instructions) {
      this.contentContainer.addChild(new Spacer(1));
      this.contentContainer.addChild(new Text(theme_exports.theme.fg("warning", instructions), 1, 0));
    }
    openBrowser(url);
    this.tui.requestRender();
  }
  /**
   * Called by onDeviceCode callback - show URL and user code.
   */
  showDeviceCode(info) {
    this.contentContainer.clear();
    this.contentContainer.addChild(new Spacer(1));
    const linkedUrl = `\x1B]8;;${info.verificationUri}\x07${info.verificationUri}\x1B]8;;\x07`;
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("accent", linkedUrl), 1, 0));
    const clickHint = process.platform === "darwin" ? "Cmd+click to open" : "Ctrl+click to open";
    const hyperlink2 = `\x1B]8;;${info.verificationUri}\x07${clickHint}\x1B]8;;\x07`;
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("dim", hyperlink2), 1, 0));
    this.contentContainer.addChild(new Spacer(1));
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("warning", `Enter code: ${info.userCode}`), 1, 0));
    this.tui.requestRender();
  }
  /**
   * Show input for manual code/URL entry (for callback server providers)
   */
  showManualInput(prompt) {
    this.input.setValue("");
    this.contentContainer.addChild(new Spacer(1));
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("dim", prompt), 1, 0));
    this.contentContainer.addChild(this.input);
    this.contentContainer.addChild(new Text(`(${keyHint("tui.select.cancel", "to cancel")})`, 1, 0));
    this.tui.requestRender();
    return new Promise((resolve5, reject) => {
      this.inputResolver = resolve5;
      this.inputRejecter = reject;
    });
  }
  /**
   * Called by onPrompt callback - show prompt and wait for input
   * Note: Does NOT clear content, appends to existing (preserves URL from showAuth)
   */
  showPrompt(message, placeholder) {
    this.contentContainer.addChild(new Spacer(1));
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("text", message), 1, 0));
    if (placeholder) {
      this.contentContainer.addChild(new Text(theme_exports.theme.fg("dim", `e.g., ${placeholder}`), 1, 0));
    }
    this.contentContainer.addChild(this.input);
    this.contentContainer.addChild(new Text(`(${keyHint("tui.select.cancel", "to cancel,")} ${keyHint("tui.select.confirm", "to submit")})`, 1, 0));
    this.input.setValue("");
    this.tui.requestRender();
    return new Promise((resolve5, reject) => {
      this.inputResolver = resolve5;
      this.inputRejecter = reject;
    });
  }
  /** Show informational text before another login step. */
  showDetails(lines) {
    this.contentContainer.clear();
    this.contentContainer.addChild(new Spacer(1));
    for (const line of lines) {
      this.contentContainer.addChild(new Text(line, 1, 0));
    }
    this.tui.requestRender();
  }
  /** Show provider-owned information and links without starting an auth callback flow. */
  showInfo(message, links = [], showCloseHint = false) {
    this.contentContainer.addChild(new Spacer(1));
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("text", message), 1, 0));
    for (const link of links) {
      const text = link.label ? `${link.label}: ${link.url}` : link.url;
      const hyperlink2 = `\x1B]8;;${link.url}\x07${text}\x1B]8;;\x07`;
      this.contentContainer.addChild(new Text(theme_exports.theme.fg("accent", hyperlink2), 1, 0));
    }
    if (showCloseHint) {
      this.contentContainer.addChild(new Spacer(1));
      this.contentContainer.addChild(new Text(`(${keyHint("tui.select.cancel", "to close")})`, 1, 0));
    }
    this.tui.requestRender();
  }
  /**
   * Show waiting message (for polling flows like GitHub Copilot)
   */
  showWaiting(message) {
    this.contentContainer.addChild(new Spacer(1));
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("dim", message), 1, 0));
    this.contentContainer.addChild(new Text(`(${keyHint("tui.select.cancel", "to cancel")})`, 1, 0));
    this.tui.requestRender();
  }
  /**
   * Called by onProgress callback
   */
  showProgress(message) {
    this.contentContainer.addChild(new Text(theme_exports.theme.fg("dim", message), 1, 0));
    this.tui.requestRender();
  }
  handleInput(data) {
    const kb = getKeybindings();
    if (kb.matches(data, "tui.select.cancel")) {
      this.cancel();
      return;
    }
    this.input.handleInput(data);
  }
};

// target/vendor-pi/dist/modes/interactive/components/model-selector.js
import { modelsAreEqual as modelsAreEqual2 } from "@earendil-works/pi-ai";

// target/vendor-pi/dist/modes/interactive/model-catalog-refresh.js
var ModelCatalogRefreshCoordinator = class {
  activeByRuntime = /* @__PURE__ */ new WeakMap();
  refresh(modelRuntime, signal) {
    signal.throwIfAborted();
    let active = this.activeByRuntime.get(modelRuntime);
    if (!active) {
      const controller = new AbortController();
      let created;
      const operation = modelRuntime.refresh({ signal: controller.signal });
      const promise = raceWithAbortSignal(operation, controller.signal).finally(() => {
        if (this.activeByRuntime.get(modelRuntime) === created) {
          this.activeByRuntime.delete(modelRuntime);
        }
      });
      created = { controller, promise, waiters: 0 };
      active = created;
      this.activeByRuntime.set(modelRuntime, active);
    }
    active.waiters++;
    return raceWithAbortSignal(active.promise, signal).finally(() => {
      active.waiters--;
      if (active.waiters === 0 && this.activeByRuntime.get(modelRuntime) === active) {
        active.controller.abort();
      }
    });
  }
};
var modelCatalogRefreshCoordinator = new ModelCatalogRefreshCoordinator();
function refreshModelCatalogs(modelRuntime, signal) {
  return modelCatalogRefreshCoordinator.refresh(modelRuntime, signal);
}

// target/vendor-pi/dist/modes/interactive/model-search.js
function getModelSelectorSearchText(item) {
  const { id, provider } = item;
  const name = item.name ? ` ${item.name}` : "";
  return `${provider} ${provider}/${id} ${provider} ${id}${name}`;
}

// target/vendor-pi/dist/modes/interactive/components/model-selector.js
var ModelSelectorComponent = class extends Container {
  searchInput;
  // Focusable implementation - propagate to searchInput for IME cursor positioning
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    this.searchInput.focused = value;
  }
  listContainer;
  allModels = [];
  scopedModelItems = [];
  activeModels = [];
  filteredModels = [];
  selectedIndex = 0;
  currentModel;
  modelRuntime;
  onSelectCallback;
  onSelectAsDefaultCallback;
  onCancelCallback;
  errorMessage;
  refreshStatusMessage = "Refreshing model catalogs\u2026";
  refreshStatusSuccess = false;
  tui;
  scopedModels;
  defaultModel;
  scope = "all";
  scopeText;
  scopeHintText;
  refreshAbortController = new AbortController();
  refreshTimeout;
  closed = false;
  constructor(tui, currentModel, modelRuntime, scopedModels, onSelect, onCancel, initialSearchInput, onSelectAsDefault, defaultModel) {
    super();
    this.tui = tui;
    this.currentModel = currentModel;
    this.modelRuntime = modelRuntime;
    this.scopedModels = scopedModels;
    this.defaultModel = defaultModel;
    this.scope = scopedModels.length > 0 ? "scoped" : "all";
    this.onSelectCallback = onSelect;
    this.onSelectAsDefaultCallback = onSelectAsDefault;
    this.onCancelCallback = onCancel;
    this.addChild(new DynamicBorder());
    this.addChild(new Spacer(1));
    if (scopedModels.length > 0) {
      this.scopeText = new Text(this.getScopeText(), 0, 0);
      this.addChild(this.scopeText);
      this.scopeHintText = new Text(this.getScopeHintText(), 0, 0);
      this.addChild(this.scopeHintText);
    } else {
      const hintText = "Only showing models from configured providers. Use /login to add providers.";
      this.addChild(new Text(theme_exports.theme.fg("warning", hintText), 0, 0));
    }
    this.addChild(new Spacer(1));
    this.searchInput = new Input();
    if (initialSearchInput) {
      this.searchInput.setValue(initialSearchInput);
    }
    this.searchInput.onSubmit = () => {
      if (this.filteredModels[this.selectedIndex]) {
        this.handleSelect(this.filteredModels[this.selectedIndex].model);
      }
    };
    this.addChild(this.searchInput);
    this.addChild(new Spacer(1));
    this.listContainer = new Container();
    this.addChild(this.listContainer);
    this.addChild(new Spacer(1));
    if (this.onSelectAsDefaultCallback) {
      this.addChild(new Text(theme_exports.theme.fg("dim", `  ${keyDisplayText("tui.select.confirm")} to select \xB7 ${keyDisplayText("app.models.save")} to set as default \xB7 ${keyDisplayText("tui.select.cancel")} to cancel`), 0, 0));
    }
    this.addChild(new DynamicBorder());
    this.loadModelsFromSnapshot();
    if (initialSearchInput)
      this.filterModels(initialSearchInput);
    else
      this.updateList();
    this.tui.requestRender();
    void this.refreshModels();
  }
  loadModelsFromSnapshot() {
    const models = this.modelRuntime.getAvailableSnapshot().map((model) => ({
      provider: model.provider,
      id: model.id,
      model
    }));
    this.allModels = this.sortModels(models);
    this.scopedModels = this.scopedModels.map((scoped) => {
      const refreshed = this.modelRuntime.getModel(scoped.model.provider, scoped.model.id);
      return refreshed ? { ...scoped, model: refreshed } : scoped;
    });
    this.scopedModelItems = this.scopedModels.map((scoped) => ({
      provider: scoped.model.provider,
      id: scoped.model.id,
      model: scoped.model
    }));
    this.activeModels = this.scope === "scoped" ? this.scopedModelItems : this.allModels;
    this.filteredModels = this.activeModels;
    const currentIndex = this.filteredModels.findIndex((item) => modelsAreEqual2(this.currentModel, item.model));
    this.selectedIndex = currentIndex >= 0 ? currentIndex : Math.min(this.selectedIndex, Math.max(0, this.filteredModels.length - 1));
  }
  async refreshModels() {
    const timeoutMs = 15e3;
    let timedOut = false;
    this.refreshTimeout = setTimeout(() => {
      timedOut = true;
      this.refreshAbortController.abort();
    }, timeoutMs);
    try {
      const result = await refreshModelCatalogs(this.modelRuntime, this.refreshAbortController.signal);
      if (this.closed)
        return;
      this.refreshStatusMessage = "";
      if (result.aborted && timedOut) {
        this.errorMessage = "Model refresh timed out; showing cached models.";
      } else if (result.errors.size === 1) {
        this.errorMessage = `Could not refresh ${result.errors.keys().next().value}; showing cached models.`;
      } else if (result.errors.size > 1) {
        this.errorMessage = `Could not refresh ${result.errors.size} model catalogs (${[...result.errors.keys()].join(", ")}); showing cached models.`;
      } else {
        this.errorMessage = this.modelRuntime.getError();
        if (!this.errorMessage) {
          this.refreshStatusMessage = "Model catalogs refreshed.";
          this.refreshStatusSuccess = true;
        }
      }
      this.loadModelsFromSnapshot();
      this.filterModels(this.searchInput.getValue());
      this.tui.requestRender();
    } catch (error) {
      if (this.closed)
        return;
      this.refreshStatusMessage = "";
      this.errorMessage = timedOut ? "Model refresh timed out; showing cached models." : `Could not refresh model catalogs: ${error instanceof Error ? error.message : String(error)}`;
      this.updateList();
      this.tui.requestRender();
    } finally {
      if (this.refreshTimeout)
        clearTimeout(this.refreshTimeout);
    }
  }
  dispose() {
    if (this.closed)
      return;
    this.closed = true;
    if (this.refreshTimeout)
      clearTimeout(this.refreshTimeout);
    this.refreshAbortController.abort();
  }
  sortModels(models) {
    const sorted = [...models];
    sorted.sort((a, b) => {
      const aIsCurrent = modelsAreEqual2(this.currentModel, a.model);
      const bIsCurrent = modelsAreEqual2(this.currentModel, b.model);
      if (aIsCurrent && !bIsCurrent)
        return -1;
      if (!aIsCurrent && bIsCurrent)
        return 1;
      const aIsDefault = this.isDefaultModel(a.model);
      const bIsDefault = this.isDefaultModel(b.model);
      if (aIsDefault && !bIsDefault)
        return -1;
      if (!aIsDefault && bIsDefault)
        return 1;
      return a.provider.localeCompare(b.provider);
    });
    return sorted;
  }
  getScopeText() {
    const allText = this.scope === "all" ? theme_exports.theme.fg("accent", "all") : theme_exports.theme.fg("muted", "all");
    const scopedText = this.scope === "scoped" ? theme_exports.theme.fg("accent", "scoped") : theme_exports.theme.fg("muted", "scoped");
    return `${theme_exports.theme.fg("muted", "Scope: ")}${allText}${theme_exports.theme.fg("muted", " | ")}${scopedText}`;
  }
  getScopeHintText() {
    return keyHint("tui.input.tab", "scope") + theme_exports.theme.fg("muted", " (all/scoped)");
  }
  isDefaultModel(model) {
    return this.defaultModel?.provider === model.provider && this.defaultModel.id === model.id;
  }
  isDefaultSearch(query) {
    const normalized = query.trim().toLowerCase();
    return normalized.length > 0 && "default".startsWith(normalized);
  }
  setScope(scope) {
    if (this.scope === scope)
      return;
    this.scope = scope;
    this.activeModels = this.scope === "scoped" ? this.scopedModelItems : this.allModels;
    const currentIndex = this.activeModels.findIndex((item) => modelsAreEqual2(this.currentModel, item.model));
    this.selectedIndex = currentIndex >= 0 ? currentIndex : 0;
    this.filterModels(this.searchInput.getValue());
    if (this.scopeText) {
      this.scopeText.setText(this.getScopeText());
    }
  }
  filterModels(query) {
    if (query) {
      const filtered = fuzzyFilter(this.activeModels, query, (item) => {
        const defaultText = this.isDefaultModel(item.model) ? " default" : "";
        return `${getModelSelectorSearchText({ id: item.id, provider: item.provider, name: item.model.name })}${defaultText}`;
      });
      if (this.isDefaultSearch(query)) {
        const defaultItems = this.activeModels.filter((item) => this.isDefaultModel(item.model));
        const defaultKeys = new Set(defaultItems.map((item) => `${item.provider}\0${item.id}`));
        this.filteredModels = [
          ...defaultItems,
          ...filtered.filter((item) => !defaultKeys.has(`${item.provider}\0${item.id}`))
        ];
      } else {
        this.filteredModels = filtered;
      }
    } else {
      this.filteredModels = this.activeModels;
    }
    this.selectedIndex = query ? 0 : Math.min(this.selectedIndex, Math.max(0, this.filteredModels.length - 1));
    this.updateList();
  }
  updateList() {
    this.listContainer.clear();
    const maxVisible = 10;
    const startIndex = Math.max(0, Math.min(this.selectedIndex - Math.floor(maxVisible / 2), this.filteredModels.length - maxVisible));
    const endIndex = Math.min(startIndex + maxVisible, this.filteredModels.length);
    for (let i = startIndex; i < endIndex; i++) {
      const item = this.filteredModels[i];
      if (!item)
        continue;
      const isSelected = i === this.selectedIndex;
      const isCurrent = modelsAreEqual2(this.currentModel, item.model);
      const isDefault = this.isDefaultModel(item.model);
      const defaultBadge = isDefault ? theme_exports.theme.fg("muted", " \xB7 default") : "";
      const cursor = isSelected ? theme_exports.theme.fg("accent", "\u2192 ") : "  ";
      const currentMarker = isCurrent ? theme_exports.theme.fg("accent", "\u2713 ") : "  ";
      const modelText = isSelected ? theme_exports.theme.fg("accent", item.id) : item.id;
      const providerBadge = theme_exports.theme.fg("muted", `[${item.provider}]`);
      const line = `${cursor}${currentMarker}${modelText} ${providerBadge}${defaultBadge}`;
      this.listContainer.addChild(new Text(line, 0, 0));
    }
    if (startIndex > 0 || endIndex < this.filteredModels.length) {
      const scrollInfo = theme_exports.theme.fg("muted", `  (${this.selectedIndex + 1}/${this.filteredModels.length})`);
      this.listContainer.addChild(new Text(scrollInfo, 0, 0));
    }
    if (this.errorMessage) {
      const errorLines = this.errorMessage.split("\n");
      for (const line of errorLines) {
        this.listContainer.addChild(new Text(theme_exports.theme.fg("error", line), 0, 0));
      }
    } else if (this.filteredModels.length === 0) {
      this.listContainer.addChild(new Text(theme_exports.theme.fg("muted", "  No matching models"), 0, 0));
    } else {
      const selected = this.filteredModels[this.selectedIndex];
      this.listContainer.addChild(new Spacer(1));
      this.listContainer.addChild(new Text(theme_exports.theme.fg("muted", `  Model Name: ${selected.model.name}`), 0, 0));
    }
    if (this.refreshStatusMessage) {
      this.listContainer.addChild(new Spacer(1));
      this.listContainer.addChild(new Text(theme_exports.theme.fg(this.refreshStatusSuccess ? "success" : "muted", `  ${this.refreshStatusMessage}`), 0, 0));
    }
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (kb.matches(keyData, "tui.input.tab")) {
      if (this.scopedModelItems.length > 0) {
        const nextScope = this.scope === "all" ? "scoped" : "all";
        this.setScope(nextScope);
        if (this.scopeHintText) {
          this.scopeHintText.setText(this.getScopeHintText());
        }
      }
      return;
    }
    if (kb.matches(keyData, "tui.select.up")) {
      if (this.filteredModels.length === 0)
        return;
      this.selectedIndex = this.selectedIndex === 0 ? this.filteredModels.length - 1 : this.selectedIndex - 1;
      this.updateList();
    } else if (kb.matches(keyData, "tui.select.down")) {
      if (this.filteredModels.length === 0)
        return;
      this.selectedIndex = this.selectedIndex === this.filteredModels.length - 1 ? 0 : this.selectedIndex + 1;
      this.updateList();
    } else if (kb.matches(keyData, "tui.select.confirm")) {
      const selectedModel = this.filteredModels[this.selectedIndex];
      if (selectedModel) {
        this.handleSelect(selectedModel.model);
      }
    } else if (kb.matches(keyData, "tui.select.cancel")) {
      this.dispose();
      this.onCancelCallback();
    } else if (kb.matches(keyData, "app.models.save") && this.onSelectAsDefaultCallback) {
      const selectedModel = this.filteredModels[this.selectedIndex];
      if (selectedModel) {
        this.dispose();
        this.onSelectAsDefaultCallback(selectedModel.model);
      }
    } else {
      this.searchInput.handleInput(keyData);
      this.filterModels(this.searchInput.getValue());
    }
  }
  handleSelect(model) {
    this.dispose();
    this.onSelectCallback(model);
  }
  getSearchInput() {
    return this.searchInput;
  }
};

// target/vendor-pi/dist/modes/interactive/components/oauth-selector.js
function formatAuthSelectorProviderType(authType, subscription) {
  if (authType === "api_key")
    return "API key";
  return subscription === false ? "account" : "subscription";
}
function formatAuthSelectorProviderStatus(provider) {
  if (!provider.status)
    return theme_exports.theme.fg("muted", " \u2022 not configured");
  if (provider.status.type !== provider.authType) {
    const label = `${formatAuthSelectorProviderType(provider.status.type, provider.subscription)} configured`;
    return theme_exports.theme.fg("muted", " \u2022 ") + theme_exports.theme.fg("warning", label);
  }
  if (!provider.status.source || provider.status.source === "OAuth" || provider.status.source === "stored credential") {
    return theme_exports.theme.fg("success", " \u2713 configured");
  }
  const source = /^[A-Z][A-Z0-9_]*(?:, [A-Z][A-Z0-9_]*)*$/.test(provider.status.source) ? `env: ${provider.status.source}` : provider.status.source;
  return theme_exports.theme.fg("success", ` \u2713 ${source}`);
}
var OAuthSelectorComponent = class extends Container {
  searchInput;
  // Focusable implementation - propagate to search input for IME cursor positioning
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    this.searchInput.focused = value;
  }
  listContainer;
  allProviders;
  filteredProviders;
  selectedIndex = 0;
  mode;
  onSelectCallback;
  onCancelCallback;
  showAuthTypeLabels;
  constructor(mode, providers, onSelect, onCancel, initialSearchInput) {
    super();
    this.mode = mode;
    this.allProviders = providers;
    this.filteredProviders = providers;
    this.showAuthTypeLabels = new Set(providers.map((provider) => provider.authType)).size > 1;
    this.onSelectCallback = onSelect;
    this.onCancelCallback = onCancel;
    this.addChild(new DynamicBorder());
    this.addChild(new Spacer(1));
    const title = mode === "login" ? "Select provider to configure:" : "Select provider to logout:";
    this.addChild(new TruncatedText(theme_exports.theme.fg("accent", theme_exports.theme.bold(title)), 1, 0));
    this.addChild(new Spacer(1));
    this.searchInput = new Input();
    if (initialSearchInput) {
      this.searchInput.setValue(initialSearchInput);
    }
    this.searchInput.onSubmit = () => {
      const selectedProvider = this.filteredProviders[this.selectedIndex];
      if (selectedProvider) {
        this.onSelectCallback(selectedProvider.id, selectedProvider.authType);
      }
    };
    this.addChild(this.searchInput);
    this.addChild(new Spacer(1));
    this.listContainer = new Container();
    this.addChild(this.listContainer);
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder());
    this.filterProviders(initialSearchInput ?? "");
  }
  filterProviders(query) {
    this.filteredProviders = query ? fuzzyFilter(this.allProviders, query, (provider) => `${provider.name} ${provider.id} ${provider.authType} ${provider.method?.name ?? ""}`) : this.allProviders;
    this.selectedIndex = Math.max(0, Math.min(this.selectedIndex, Math.max(0, this.filteredProviders.length - 1)));
    this.updateList();
  }
  updateList() {
    this.listContainer.clear();
    const maxVisible = 8;
    const startIndex = Math.max(0, Math.min(this.selectedIndex - Math.floor(maxVisible / 2), this.filteredProviders.length - maxVisible));
    const endIndex = Math.min(startIndex + maxVisible, this.filteredProviders.length);
    for (let i = startIndex; i < endIndex; i++) {
      const provider = this.filteredProviders[i];
      if (!provider)
        continue;
      const isSelected = i === this.selectedIndex;
      const statusIndicator = formatAuthSelectorProviderStatus(provider);
      const authTypeLabel = this.showAuthTypeLabels ? theme_exports.theme.fg("muted", ` [${formatAuthSelectorProviderType(provider.authType, provider.subscription)}]`) : "";
      let line = "";
      if (isSelected) {
        const prefix = theme_exports.theme.fg("accent", "\u2192 ");
        const text = theme_exports.theme.fg("accent", provider.name);
        line = prefix + text + authTypeLabel + statusIndicator;
      } else {
        const text = `  ${theme_exports.theme.fg("text", provider.name)}`;
        line = text + authTypeLabel + statusIndicator;
      }
      this.listContainer.addChild(new TruncatedText(line, 1, 0));
    }
    if (startIndex > 0 || endIndex < this.filteredProviders.length) {
      const scrollInfo = theme_exports.theme.fg("muted", `  (${this.selectedIndex + 1}/${this.filteredProviders.length})`);
      this.listContainer.addChild(new TruncatedText(scrollInfo, 1, 0));
    }
    if (this.filteredProviders.length === 0) {
      const message = this.allProviders.length === 0 ? this.mode === "login" ? "No providers available" : "No providers logged in. Use /login first." : "No matching providers";
      this.listContainer.addChild(new TruncatedText(theme_exports.theme.fg("muted", `  ${message}`), 1, 0));
    }
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (kb.matches(keyData, "tui.select.up")) {
      if (this.filteredProviders.length === 0)
        return;
      this.selectedIndex = Math.max(0, this.selectedIndex - 1);
      this.updateList();
    } else if (kb.matches(keyData, "tui.select.down")) {
      if (this.filteredProviders.length === 0)
        return;
      this.selectedIndex = Math.min(this.filteredProviders.length - 1, this.selectedIndex + 1);
      this.updateList();
    } else if (kb.matches(keyData, "tui.select.confirm")) {
      const selectedProvider = this.filteredProviders[this.selectedIndex];
      if (selectedProvider) {
        this.onSelectCallback(selectedProvider.id, selectedProvider.authType);
      }
    } else if (kb.matches(keyData, "tui.select.cancel")) {
      this.onCancelCallback();
    } else {
      this.searchInput.handleInput(keyData);
      this.filterProviders(this.searchInput.getValue());
    }
  }
};

// target/vendor-pi/dist/modes/interactive/components/session-selector.js
import { spawnSync as spawnSync4 } from "node:child_process";
import { existsSync as existsSync9 } from "node:fs";
import { unlink } from "node:fs/promises";
import * as os2 from "node:os";

// target/vendor-pi/dist/core/keybindings.js
import { existsSync as existsSync8, readFileSync as readFileSync6 } from "fs";
import { join as join11 } from "path";
function useWindowsKeybindings(platform2 = process.platform, env = process.env) {
  return platform2 === "win32" || platform2 === "linux" && Boolean(env.WSL_DISTRO_NAME || env.WSL_INTEROP);
}
var windowsKeybindings = useWindowsKeybindings();
var KEYBINDINGS = {
  ...TUI_KEYBINDINGS,
  "tui.editor.undo": {
    ...TUI_KEYBINDINGS["tui.editor.undo"],
    defaultKeys: process.platform === "win32" ? "ctrl+z" : windowsKeybindings ? "alt+z" : "ctrl+-"
  },
  "tui.altScreen.previousPrompt": {
    ...TUI_KEYBINDINGS["tui.altScreen.previousPrompt"],
    defaultKeys: windowsKeybindings ? "ctrl+up" : ["ctrl+shift+up", "ctrl+up"]
  },
  "tui.altScreen.nextPrompt": {
    ...TUI_KEYBINDINGS["tui.altScreen.nextPrompt"],
    defaultKeys: windowsKeybindings ? "ctrl+down" : ["ctrl+shift+down", "ctrl+down"]
  },
  "tui.altScreen.search": {
    ...TUI_KEYBINDINGS["tui.altScreen.search"],
    defaultKeys: windowsKeybindings ? "ctrl+f" : "ctrl+shift+f"
  },
  "app.interrupt": { defaultKeys: "escape", description: "Cancel or abort" },
  "app.clear": { defaultKeys: "ctrl+c", description: "Clear editor" },
  "app.exit": { defaultKeys: "ctrl+d", description: "Exit when editor is empty" },
  "app.suspend": {
    defaultKeys: process.platform === "win32" ? [] : "ctrl+z",
    description: "Suspend to background"
  },
  "app.thinking.cycle": {
    defaultKeys: "shift+tab",
    description: "Cycle thinking level"
  },
  "app.thinking.save": {
    defaultKeys: "ctrl+s",
    description: "Save thinking level"
  },
  "app.model.cycleForward": {
    defaultKeys: "ctrl+p",
    description: "Cycle to next model"
  },
  "app.model.cycleBackward": {
    defaultKeys: windowsKeybindings ? "alt+p" : "shift+ctrl+p",
    description: "Cycle to previous model"
  },
  "app.model.select": { defaultKeys: "ctrl+l", description: "Open model selector" },
  "app.tools.expand": { defaultKeys: "ctrl+o", description: "Toggle tool output" },
  "app.thinking.toggle": {
    defaultKeys: "ctrl+t",
    description: "Toggle thinking blocks"
  },
  "app.session.toggleNamedFilter": {
    defaultKeys: "ctrl+n",
    description: "Toggle named session filter"
  },
  "app.editor.external": {
    defaultKeys: "ctrl+g",
    description: "Open external editor"
  },
  "app.message.copy": {
    defaultKeys: "ctrl+x",
    description: "Copy selection or last assistant message"
  },
  "app.message.followUp": {
    defaultKeys: windowsKeybindings ? "ctrl+q" : "alt+enter",
    description: "Queue follow-up message"
  },
  "app.message.dequeue": {
    defaultKeys: windowsKeybindings ? "alt+q" : "alt+up",
    description: "Restore queued messages"
  },
  "app.clipboard.pasteImage": {
    defaultKeys: windowsKeybindings ? "alt+v" : "ctrl+v",
    description: "Paste files on macOS, images, or text from clipboard"
  },
  "app.session.new": { defaultKeys: [], description: "Start a new session" },
  "app.session.tree": { defaultKeys: [], description: "Open session tree" },
  "app.session.fork": { defaultKeys: [], description: "Fork current session" },
  "app.session.resume": { defaultKeys: [], description: "Resume a session" },
  "app.tree.foldOrUp": {
    defaultKeys: process.platform === "darwin" ? ["alt+left", "ctrl+left"] : ["ctrl+left", "alt+left"],
    description: "Fold tree branch or move up"
  },
  "app.tree.unfoldOrDown": {
    defaultKeys: process.platform === "darwin" ? ["alt+right", "ctrl+right"] : ["ctrl+right", "alt+right"],
    description: "Unfold tree branch or move down"
  },
  "app.tree.editLabel": {
    defaultKeys: "shift+l",
    description: "Edit tree label"
  },
  "app.tree.toggleLabelTimestamp": {
    defaultKeys: "shift+t",
    description: "Toggle tree label timestamps"
  },
  "app.session.togglePath": {
    defaultKeys: "ctrl+p",
    description: "Toggle session path display"
  },
  "app.session.toggleSort": {
    defaultKeys: "ctrl+s",
    description: "Toggle session sort mode"
  },
  "app.session.rename": {
    defaultKeys: "ctrl+r",
    description: "Rename session"
  },
  "app.session.delete": {
    defaultKeys: "ctrl+d",
    description: "Delete session"
  },
  "app.session.deleteNoninvasive": {
    defaultKeys: "ctrl+backspace",
    description: "Delete session when query is empty"
  },
  "app.models.save": {
    defaultKeys: "ctrl+s",
    description: "Save model selection"
  },
  "app.models.enableAll": {
    defaultKeys: "ctrl+a",
    description: "Enable all models"
  },
  "app.models.clearAll": {
    defaultKeys: "ctrl+x",
    description: "Clear all models"
  },
  "app.models.toggleProvider": {
    defaultKeys: "ctrl+p",
    description: "Toggle all models for provider"
  },
  "app.models.reorderUp": {
    defaultKeys: "alt+up",
    description: "Move model up in order"
  },
  "app.models.reorderDown": {
    defaultKeys: "alt+down",
    description: "Move model down in order"
  },
  "app.tree.filter.default": {
    defaultKeys: "ctrl+d",
    description: "Tree filter: default view"
  },
  "app.tree.filter.noTools": {
    defaultKeys: "ctrl+t",
    description: "Tree filter: hide tool results"
  },
  "app.tree.filter.userOnly": {
    defaultKeys: "ctrl+u",
    description: "Tree filter: user messages only"
  },
  "app.tree.filter.labeledOnly": {
    defaultKeys: "ctrl+l",
    description: "Tree filter: labeled entries only"
  },
  "app.tree.filter.all": {
    defaultKeys: "ctrl+a",
    description: "Tree filter: show all entries"
  },
  "app.tree.filter.cycleForward": {
    defaultKeys: "ctrl+o",
    description: "Tree filter: cycle forward"
  },
  "app.tree.filter.cycleBackward": {
    defaultKeys: "shift+ctrl+o",
    description: "Tree filter: cycle backward"
  }
};
var KEYBINDING_NAME_MIGRATIONS = {
  cursorUp: "tui.editor.cursorUp",
  cursorDown: "tui.editor.cursorDown",
  cursorLeft: "tui.editor.cursorLeft",
  cursorRight: "tui.editor.cursorRight",
  cursorWordLeft: "tui.editor.cursorWordLeft",
  cursorWordRight: "tui.editor.cursorWordRight",
  cursorLineStart: "tui.editor.cursorLineStart",
  cursorLineEnd: "tui.editor.cursorLineEnd",
  jumpForward: "tui.editor.jumpForward",
  jumpBackward: "tui.editor.jumpBackward",
  pageUp: "tui.editor.pageUp",
  pageDown: "tui.editor.pageDown",
  deleteCharBackward: "tui.editor.deleteCharBackward",
  deleteCharForward: "tui.editor.deleteCharForward",
  deleteWordBackward: "tui.editor.deleteWordBackward",
  deleteWordForward: "tui.editor.deleteWordForward",
  deleteToLineStart: "tui.editor.deleteToLineStart",
  deleteToLineEnd: "tui.editor.deleteToLineEnd",
  yank: "tui.editor.yank",
  yankPop: "tui.editor.yankPop",
  undo: "tui.editor.undo",
  newLine: "tui.input.newLine",
  submit: "tui.input.submit",
  tab: "tui.input.tab",
  copy: "tui.input.copy",
  selectUp: "tui.select.up",
  selectDown: "tui.select.down",
  selectPageUp: "tui.select.pageUp",
  selectPageDown: "tui.select.pageDown",
  selectConfirm: "tui.select.confirm",
  selectCancel: "tui.select.cancel",
  interrupt: "app.interrupt",
  clear: "app.clear",
  exit: "app.exit",
  suspend: "app.suspend",
  cycleThinkingLevel: "app.thinking.cycle",
  cycleModelForward: "app.model.cycleForward",
  cycleModelBackward: "app.model.cycleBackward",
  selectModel: "app.model.select",
  expandTools: "app.tools.expand",
  toggleThinking: "app.thinking.toggle",
  toggleSessionNamedFilter: "app.session.toggleNamedFilter",
  externalEditor: "app.editor.external",
  followUp: "app.message.followUp",
  dequeue: "app.message.dequeue",
  pasteImage: "app.clipboard.pasteImage",
  newSession: "app.session.new",
  tree: "app.session.tree",
  fork: "app.session.fork",
  resume: "app.session.resume",
  treeFoldOrUp: "app.tree.foldOrUp",
  treeUnfoldOrDown: "app.tree.unfoldOrDown",
  treeEditLabel: "app.tree.editLabel",
  treeToggleLabelTimestamp: "app.tree.toggleLabelTimestamp",
  toggleSessionPath: "app.session.togglePath",
  toggleSessionSort: "app.session.toggleSort",
  renameSession: "app.session.rename",
  deleteSession: "app.session.delete",
  deleteSessionNoninvasive: "app.session.deleteNoninvasive"
};
function isLegacyKeybindingName(key) {
  return key in KEYBINDING_NAME_MIGRATIONS;
}
function toKeybindingsConfig(value) {
  const config = {};
  for (const [key, binding] of Object.entries(value)) {
    if (typeof binding === "string") {
      config[key] = binding;
      continue;
    }
    if (Array.isArray(binding) && binding.every((entry) => typeof entry === "string")) {
      config[key] = binding;
    }
  }
  return config;
}
function migrateKeybindingsConfig(rawConfig) {
  const config = {};
  let migrated = false;
  for (const [key, value] of Object.entries(rawConfig)) {
    const nextKey = isLegacyKeybindingName(key) ? KEYBINDING_NAME_MIGRATIONS[key] : key;
    if (nextKey !== key) {
      migrated = true;
    }
    if (key !== nextKey && Object.hasOwn(rawConfig, nextKey)) {
      migrated = true;
      continue;
    }
    config[nextKey] = value;
  }
  return { config: orderKeybindingsConfig(config), migrated };
}
function orderKeybindingsConfig(config) {
  const ordered = {};
  for (const keybinding of Object.keys(KEYBINDINGS)) {
    if (Object.hasOwn(config, keybinding)) {
      ordered[keybinding] = config[keybinding];
    }
  }
  const extras = Object.keys(config).filter((key) => !Object.hasOwn(ordered, key)).sort();
  for (const key of extras) {
    ordered[key] = config[key];
  }
  return ordered;
}
function loadRawConfig(path4) {
  if (!existsSync8(path4))
    return void 0;
  try {
    const parsed = JSON.parse(stripBom(readFileSync6(path4, "utf-8")));
    if (typeof parsed !== "object" || parsed === null)
      return void 0;
    return parsed;
  } catch {
    return void 0;
  }
}
var KeybindingsManager2 = class _KeybindingsManager extends KeybindingsManager {
  configPath;
  constructor(userBindings = {}, configPath) {
    super(KEYBINDINGS, userBindings);
    this.configPath = configPath;
  }
  static create(agentDir = (0, config_exports.getAgentDir)()) {
    const configPath = join11(agentDir, "keybindings.json");
    const userBindings = _KeybindingsManager.loadFromFile(configPath);
    return new _KeybindingsManager(userBindings, configPath);
  }
  reload() {
    if (!this.configPath)
      return;
    this.setUserBindings(_KeybindingsManager.loadFromFile(this.configPath));
  }
  getEffectiveConfig() {
    return this.getResolvedBindings();
  }
  static loadFromFile(path4) {
    const rawConfig = loadRawConfig(path4);
    if (!rawConfig)
      return {};
    return toKeybindingsConfig(migrateKeybindingsConfig(rawConfig).config);
  }
};

// target/vendor-pi/dist/modes/interactive/components/session-selector-search.js
function normalizeWhitespaceLower(text) {
  return text.toLowerCase().replace(/\s+/g, " ").trim();
}
function getSessionSearchText(session) {
  return `${session.id} ${session.name ?? ""} ${session.allMessagesText} ${session.cwd}`;
}
function hasSessionName(session) {
  return Boolean(session.name?.trim());
}
function matchesNameFilter(session, filter2) {
  if (filter2 === "all")
    return true;
  return hasSessionName(session);
}
function parseSearchQuery(query) {
  const trimmed = query.trim();
  if (!trimmed) {
    return { mode: "tokens", tokens: [], regex: null };
  }
  if (trimmed.startsWith("re:")) {
    const pattern = trimmed.slice(3).trim();
    if (!pattern) {
      return { mode: "regex", tokens: [], regex: null, error: "Empty regex" };
    }
    try {
      return { mode: "regex", tokens: [], regex: new RegExp(pattern, "i") };
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      return { mode: "regex", tokens: [], regex: null, error: msg };
    }
  }
  const tokens = [];
  let buf = "";
  let inQuote = false;
  let hadUnclosedQuote = false;
  const flush = (kind) => {
    const v = buf.trim();
    buf = "";
    if (!v)
      return;
    tokens.push({ kind, value: v });
  };
  for (let i = 0; i < trimmed.length; i++) {
    const ch = trimmed[i];
    if (ch === '"') {
      if (inQuote) {
        flush("phrase");
        inQuote = false;
      } else {
        flush("fuzzy");
        inQuote = true;
      }
      continue;
    }
    if (!inQuote && /\s/.test(ch)) {
      flush("fuzzy");
      continue;
    }
    buf += ch;
  }
  if (inQuote) {
    hadUnclosedQuote = true;
  }
  if (hadUnclosedQuote) {
    return {
      mode: "tokens",
      tokens: trimmed.split(/\s+/).map((t) => t.trim()).filter((t) => t.length > 0).map((t) => ({ kind: "fuzzy", value: t })),
      regex: null
    };
  }
  flush(inQuote ? "phrase" : "fuzzy");
  return { mode: "tokens", tokens, regex: null };
}
function matchSession(session, parsed) {
  const text = getSessionSearchText(session);
  if (parsed.mode === "regex") {
    if (!parsed.regex) {
      return { matches: false, score: 0 };
    }
    const idx = text.search(parsed.regex);
    if (idx < 0)
      return { matches: false, score: 0 };
    return { matches: true, score: idx * 0.1 };
  }
  if (parsed.tokens.length === 0) {
    return { matches: true, score: 0 };
  }
  let totalScore = 0;
  let normalizedText = null;
  for (const token of parsed.tokens) {
    if (token.kind === "phrase") {
      if (normalizedText === null) {
        normalizedText = normalizeWhitespaceLower(text);
      }
      const phrase = normalizeWhitespaceLower(token.value);
      if (!phrase)
        continue;
      const idx = normalizedText.indexOf(phrase);
      if (idx < 0)
        return { matches: false, score: 0 };
      totalScore += idx * 0.1;
      continue;
    }
    const m = fuzzyMatch(token.value, text);
    if (!m.matches)
      return { matches: false, score: 0 };
    totalScore += m.score;
  }
  return { matches: true, score: totalScore };
}
function filterAndSortSessions(sessions, query, sortMode, nameFilter = "all") {
  const nameFiltered = nameFilter === "all" ? sessions : sessions.filter((session) => matchesNameFilter(session, nameFilter));
  const trimmed = query.trim();
  if (!trimmed)
    return nameFiltered;
  const parsed = parseSearchQuery(query);
  if (parsed.error)
    return [];
  if (sortMode === "recent") {
    const filtered = [];
    for (const s of nameFiltered) {
      const res = matchSession(s, parsed);
      if (res.matches)
        filtered.push(s);
    }
    return filtered;
  }
  const scored = [];
  for (const s of nameFiltered) {
    const res = matchSession(s, parsed);
    if (!res.matches)
      continue;
    scored.push({ session: s, score: res.score });
  }
  scored.sort((a, b) => {
    if (a.score !== b.score)
      return a.score - b.score;
    return b.session.modified.getTime() - a.session.modified.getTime();
  });
  return scored.map((r) => r.session);
}

// target/vendor-pi/dist/modes/interactive/components/session-selector.js
function shortenPath2(path4) {
  const home = os2.homedir();
  if (!path4)
    return path4;
  if (path4.startsWith(home)) {
    return `~${path4.slice(home.length)}`;
  }
  return path4;
}
function formatSessionDate(date) {
  const now = /* @__PURE__ */ new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 6e4);
  const diffHours = Math.floor(diffMs / 36e5);
  const diffDays = Math.floor(diffMs / 864e5);
  if (diffMins < 1)
    return "now";
  if (diffMins < 60)
    return `${diffMins}m`;
  if (diffHours < 24)
    return `${diffHours}h`;
  if (diffDays < 7)
    return `${diffDays}d`;
  if (diffDays < 30)
    return `${Math.floor(diffDays / 7)}w`;
  if (diffDays < 365)
    return `${Math.floor(diffDays / 30)}mo`;
  return `${Math.floor(diffDays / 365)}y`;
}
function canonicalizePath2(path4) {
  if (!path4)
    return path4;
  return canonicalizePath(path4);
}
var SessionSelectorHeader = class {
  scope;
  sortMode;
  nameFilter;
  requestRender;
  loading = false;
  loadProgress = null;
  showPath = false;
  confirmingDeletePath = null;
  statusMessage = null;
  statusTimeout = null;
  showRenameHint = false;
  constructor(scope, sortMode, nameFilter, requestRender) {
    this.scope = scope;
    this.sortMode = sortMode;
    this.nameFilter = nameFilter;
    this.requestRender = requestRender;
  }
  setScope(scope) {
    this.scope = scope;
  }
  setSortMode(sortMode) {
    this.sortMode = sortMode;
  }
  setNameFilter(nameFilter) {
    this.nameFilter = nameFilter;
  }
  setLoading(loading) {
    this.loading = loading;
    this.loadProgress = null;
  }
  setProgress(loaded, total) {
    this.loadProgress = { loaded, total };
  }
  setShowPath(showPath) {
    this.showPath = showPath;
  }
  setShowRenameHint(show) {
    this.showRenameHint = show;
  }
  setConfirmingDeletePath(path4) {
    this.confirmingDeletePath = path4;
  }
  clearStatusTimeout() {
    if (!this.statusTimeout)
      return;
    clearTimeout(this.statusTimeout);
    this.statusTimeout = null;
  }
  setStatusMessage(msg, autoHideMs) {
    this.clearStatusTimeout();
    this.statusMessage = msg;
    if (!msg || !autoHideMs)
      return;
    this.statusTimeout = setTimeout(() => {
      this.statusMessage = null;
      this.statusTimeout = null;
      this.requestRender();
    }, autoHideMs);
  }
  invalidate() {
  }
  render(width) {
    const title = this.scope === "current" ? "Resume Session (Current Folder)" : "Resume Session (All)";
    const leftText = theme_exports.theme.bold(title);
    const sortLabel = this.sortMode === "threaded" ? "Threaded" : this.sortMode === "recent" ? "Recent" : "Fuzzy";
    const sortText = theme_exports.theme.fg("muted", "Sort: ") + theme_exports.theme.fg("accent", sortLabel);
    const nameLabel = this.nameFilter === "all" ? "All" : "Named";
    const nameText = theme_exports.theme.fg("muted", "Name: ") + theme_exports.theme.fg("accent", nameLabel);
    let scopeText;
    if (this.loading) {
      const progressText = this.loadProgress ? `${this.loadProgress.loaded}/${this.loadProgress.total}` : "...";
      scopeText = `${theme_exports.theme.fg("muted", "\u25CB Current Folder | ")}${theme_exports.theme.fg("accent", `Loading ${progressText}`)}`;
    } else if (this.scope === "current") {
      scopeText = `${theme_exports.theme.fg("accent", "\u25C9 Current Folder")}${theme_exports.theme.fg("muted", " | \u25CB All")}`;
    } else {
      scopeText = `${theme_exports.theme.fg("muted", "\u25CB Current Folder | ")}${theme_exports.theme.fg("accent", "\u25C9 All")}`;
    }
    const rightText = truncateToWidth(`${scopeText}  ${nameText}  ${sortText}`, width, "");
    const availableLeft = Math.max(0, width - visibleWidth(rightText) - 1);
    const left = truncateToWidth(leftText, availableLeft, "");
    const spacing = Math.max(0, width - visibleWidth(left) - visibleWidth(rightText));
    let hintLine1;
    let hintLine2;
    if (this.confirmingDeletePath !== null) {
      const confirmHint = `Delete session? ${keyHint("tui.select.confirm", "confirm")} \xB7 ${keyHint("tui.select.cancel", "cancel")}`;
      hintLine1 = theme_exports.theme.fg("error", truncateToWidth(confirmHint, width, "\u2026"));
      hintLine2 = "";
    } else if (this.statusMessage) {
      const color = this.statusMessage.type === "error" ? "error" : "accent";
      hintLine1 = theme_exports.theme.fg(color, truncateToWidth(this.statusMessage.message, width, "\u2026"));
      hintLine2 = "";
    } else {
      const pathState = this.showPath ? "(on)" : "(off)";
      const sep6 = theme_exports.theme.fg("muted", " \xB7 ");
      const hint1 = keyHint("tui.input.tab", "scope") + sep6 + theme_exports.theme.fg("muted", 're:<pattern> regex \xB7 "phrase" exact');
      const hint2Parts = [
        keyHint("app.session.toggleSort", "sort"),
        keyHint("app.session.toggleNamedFilter", "named"),
        keyHint("app.session.delete", "delete"),
        keyHint("app.session.togglePath", `path ${pathState}`)
      ];
      if (this.showRenameHint) {
        hint2Parts.push(keyHint("app.session.rename", "rename"));
      }
      const hint2 = hint2Parts.join(sep6);
      hintLine1 = truncateToWidth(hint1, width, "\u2026");
      hintLine2 = truncateToWidth(hint2, width, "\u2026");
    }
    return [`${left}${" ".repeat(spacing)}${rightText}`, hintLine1, hintLine2];
  }
};
function buildSessionTree(sessions) {
  const byPath = /* @__PURE__ */ new Map();
  for (const session of sessions) {
    const sessionPath = canonicalizePath2(session.path) ?? session.path;
    byPath.set(sessionPath, { session, children: [], latestActivity: session.modified.getTime() });
  }
  const roots = [];
  for (const session of sessions) {
    const sessionPath = canonicalizePath2(session.path) ?? session.path;
    const node = byPath.get(sessionPath);
    const parentPath = canonicalizePath2(session.parentSessionPath);
    if (parentPath && byPath.has(parentPath)) {
      byPath.get(parentPath).children.push(node);
    } else {
      roots.push(node);
    }
  }
  const updateLatestActivity = (node) => {
    let latestActivity = node.session.modified.getTime();
    for (const child of node.children) {
      latestActivity = Math.max(latestActivity, updateLatestActivity(child));
    }
    node.latestActivity = latestActivity;
    return latestActivity;
  };
  for (const root of roots) {
    updateLatestActivity(root);
  }
  const sortNodes = (nodes) => {
    nodes.sort((a, b) => b.latestActivity - a.latestActivity);
    for (const node of nodes) {
      sortNodes(node.children);
    }
  };
  sortNodes(roots);
  return roots;
}
function flattenSessionTree(roots) {
  const result = [];
  const walk = (node, depth, ancestorContinues, isLast) => {
    result.push({ session: node.session, depth, isLast, ancestorContinues });
    for (let i = 0; i < node.children.length; i++) {
      const childIsLast = i === node.children.length - 1;
      const continues = depth > 0 ? !isLast : false;
      walk(node.children[i], depth + 1, [...ancestorContinues, continues], childIsLast);
    }
  };
  for (let i = 0; i < roots.length; i++) {
    walk(roots[i], 0, [], i === roots.length - 1);
  }
  return result;
}
var SessionList = class {
  getSelectedSessionPath() {
    const selected = this.filteredSessions[this.selectedIndex];
    return selected?.session.path;
  }
  allSessions = [];
  filteredSessions = [];
  selectedIndex = 0;
  selectionTouched = false;
  searchInput;
  showCwd = false;
  sortMode = "threaded";
  nameFilter = "all";
  keybindings;
  showPath = false;
  confirmingDeletePath = null;
  currentSessionCanonicalPath;
  onSelect;
  onCancel;
  onExit = () => {
  };
  onToggleScope;
  onToggleSort;
  onToggleNameFilter;
  onTogglePath;
  onDeleteConfirmationChange;
  onDeleteSession;
  onRenameSession;
  onError;
  maxVisible = 10;
  // Max sessions visible (one line each)
  // Focusable implementation - propagate to searchInput for IME cursor positioning
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    this.searchInput.focused = value;
  }
  constructor(sessions, showCwd, sortMode, nameFilter, keybindings, currentSessionFilePath) {
    this.allSessions = sessions;
    this.filteredSessions = [];
    this.searchInput = new Input();
    this.showCwd = showCwd;
    this.sortMode = sortMode;
    this.nameFilter = nameFilter;
    this.keybindings = keybindings;
    this.currentSessionCanonicalPath = canonicalizePath2(currentSessionFilePath);
    this.filterSessions("");
    this.searchInput.onSubmit = () => {
      if (this.filteredSessions[this.selectedIndex]) {
        const selected = this.filteredSessions[this.selectedIndex];
        if (this.onSelect) {
          this.onSelect(selected.session.path);
        }
      }
    };
  }
  setSortMode(sortMode) {
    this.sortMode = sortMode;
    this.filterSessions(this.searchInput.getValue());
  }
  setNameFilter(nameFilter) {
    this.nameFilter = nameFilter;
    this.filterSessions(this.searchInput.getValue());
  }
  setSessions(sessions, showCwd) {
    const selectedPath = this.selectionTouched ? this.getSelectedSessionPath() : void 0;
    this.allSessions = sessions;
    this.showCwd = showCwd;
    this.filterSessions(this.searchInput.getValue());
    if (!this.selectionTouched) {
      this.selectedIndex = 0;
    } else if (selectedPath) {
      const selectedIndex = this.filteredSessions.findIndex((node) => node.session.path === selectedPath);
      if (selectedIndex >= 0)
        this.selectedIndex = selectedIndex;
    }
  }
  filterSessions(query) {
    const trimmed = query.trim();
    const nameFiltered = this.nameFilter === "all" ? this.allSessions : this.allSessions.filter((session) => hasSessionName(session));
    if (this.sortMode === "threaded" && !trimmed) {
      const roots = buildSessionTree(nameFiltered);
      this.filteredSessions = flattenSessionTree(roots);
    } else {
      const filtered = filterAndSortSessions(nameFiltered, query, this.sortMode, "all");
      this.filteredSessions = filtered.map((session) => ({
        session,
        depth: 0,
        isLast: true,
        ancestorContinues: []
      }));
    }
    this.selectedIndex = Math.min(this.selectedIndex, Math.max(0, this.filteredSessions.length - 1));
  }
  setConfirmingDeletePath(path4) {
    this.confirmingDeletePath = path4;
    this.onDeleteConfirmationChange?.(path4);
  }
  startDeleteConfirmationForSelectedSession() {
    const selected = this.filteredSessions[this.selectedIndex];
    if (!selected)
      return;
    if (this.isCurrentSessionPath(selected.session.path)) {
      this.onError?.("Cannot delete the currently active session");
      return;
    }
    this.setConfirmingDeletePath(selected.session.path);
  }
  isCurrentSessionPath(path4) {
    if (!this.currentSessionCanonicalPath)
      return false;
    return (canonicalizePath2(path4) ?? path4) === this.currentSessionCanonicalPath;
  }
  invalidate() {
  }
  render(width) {
    const lines = [];
    lines.push(...this.searchInput.render(width));
    lines.push("");
    if (this.filteredSessions.length === 0) {
      let emptyMessage;
      if (this.nameFilter === "named") {
        const toggleKey = keyText("app.session.toggleNamedFilter");
        if (this.showCwd) {
          emptyMessage = `  No named sessions found. Press ${toggleKey} to show all.`;
        } else {
          emptyMessage = `  No named sessions in current folder. Press ${toggleKey} to show all, or Tab to view all.`;
        }
      } else if (this.showCwd) {
        emptyMessage = "  No sessions found";
      } else {
        emptyMessage = "  No sessions in current folder. Press Tab to view all.";
      }
      lines.push(theme_exports.theme.fg("muted", truncateToWidth(emptyMessage, width, "\u2026")));
      return lines;
    }
    const startIndex = Math.max(0, Math.min(this.selectedIndex - Math.floor(this.maxVisible / 2), this.filteredSessions.length - this.maxVisible));
    const endIndex = Math.min(startIndex + this.maxVisible, this.filteredSessions.length);
    for (let i = startIndex; i < endIndex; i++) {
      const node = this.filteredSessions[i];
      const session = node.session;
      const isSelected = i === this.selectedIndex;
      const isConfirmingDelete = session.path === this.confirmingDeletePath;
      const isCurrent = this.isCurrentSessionPath(session.path);
      const prefix = this.buildTreePrefix(node);
      const hasName = !!session.name;
      const displayText = session.name ?? session.firstMessage;
      const normalizedMessage = displayText.replace(/[\x00-\x1f\x7f]/g, " ").trim();
      const age = formatSessionDate(session.modified);
      const msgCount = String(session.messageCount);
      let rightPart = `${msgCount} ${age}`;
      if (this.showCwd && session.cwd) {
        rightPart = `${shortenPath2(session.cwd)} ${rightPart}`;
      }
      if (this.showPath) {
        rightPart = `${shortenPath2(session.path)} ${rightPart}`;
      }
      const cursor = isSelected ? theme_exports.theme.fg("accent", "\u203A ") : "  ";
      const prefixWidth = visibleWidth(prefix);
      const rightWidth = visibleWidth(rightPart) + 2;
      const availableForMsg = width - 2 - prefixWidth - rightWidth;
      const truncatedMsg = truncateToWidth(normalizedMessage, Math.max(10, availableForMsg), "\u2026");
      let messageColor = null;
      if (isConfirmingDelete) {
        messageColor = "error";
      } else if (isCurrent) {
        messageColor = "accent";
      } else if (hasName) {
        messageColor = "warning";
      }
      let styledMsg = messageColor ? theme_exports.theme.fg(messageColor, truncatedMsg) : truncatedMsg;
      if (isSelected) {
        styledMsg = theme_exports.theme.bold(styledMsg);
      }
      const leftPart = cursor + theme_exports.theme.fg("dim", prefix) + styledMsg;
      const leftWidth = visibleWidth(leftPart);
      const spacing = Math.max(1, width - leftWidth - visibleWidth(rightPart));
      const styledRight = theme_exports.theme.fg(isConfirmingDelete ? "error" : "dim", rightPart);
      let line = leftPart + " ".repeat(spacing) + styledRight;
      if (isSelected) {
        line = theme_exports.theme.bg("selectedBg", line);
      }
      lines.push(truncateToWidth(line, width));
    }
    if (startIndex > 0 || endIndex < this.filteredSessions.length) {
      const scrollText = `  (${this.selectedIndex + 1}/${this.filteredSessions.length})`;
      const scrollInfo = theme_exports.theme.fg("muted", truncateToWidth(scrollText, width, ""));
      lines.push(scrollInfo);
    }
    return lines;
  }
  buildTreePrefix(node) {
    if (node.depth === 0) {
      return "";
    }
    const parts = node.ancestorContinues.map((continues) => continues ? "\u2502  " : "   ");
    const branch = node.isLast ? "\u2514\u2500 " : "\u251C\u2500 ";
    return parts.join("") + branch;
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (this.confirmingDeletePath !== null) {
      if (kb.matches(keyData, "tui.select.confirm")) {
        const pathToDelete = this.confirmingDeletePath;
        this.setConfirmingDeletePath(null);
        void this.onDeleteSession?.(pathToDelete);
        return;
      }
      if (kb.matches(keyData, "tui.select.cancel")) {
        this.setConfirmingDeletePath(null);
        return;
      }
      return;
    }
    if (kb.matches(keyData, "tui.input.tab")) {
      if (this.onToggleScope) {
        this.onToggleScope();
      }
      return;
    }
    if (kb.matches(keyData, "app.session.toggleSort")) {
      this.onToggleSort?.();
      return;
    }
    if (this.keybindings.matches(keyData, "app.session.toggleNamedFilter")) {
      this.onToggleNameFilter?.();
      return;
    }
    if (kb.matches(keyData, "app.session.togglePath")) {
      this.showPath = !this.showPath;
      this.onTogglePath?.(this.showPath);
      return;
    }
    if (kb.matches(keyData, "app.session.delete")) {
      this.startDeleteConfirmationForSelectedSession();
      return;
    }
    if (kb.matches(keyData, "app.session.rename")) {
      const selected = this.filteredSessions[this.selectedIndex];
      if (selected) {
        this.onRenameSession?.(selected.session.path);
      }
      return;
    }
    if (kb.matches(keyData, "app.session.deleteNoninvasive")) {
      if (this.searchInput.getValue().length > 0) {
        this.searchInput.handleInput(keyData);
        this.filterSessions(this.searchInput.getValue());
        return;
      }
      this.startDeleteConfirmationForSelectedSession();
      return;
    }
    this.selectionTouched = true;
    if (kb.matches(keyData, "tui.select.up")) {
      this.selectedIndex = Math.max(0, this.selectedIndex - 1);
    } else if (kb.matches(keyData, "tui.select.down")) {
      this.selectedIndex = Math.min(this.filteredSessions.length - 1, this.selectedIndex + 1);
    } else if (kb.matches(keyData, "tui.select.pageUp")) {
      this.selectedIndex = Math.max(0, this.selectedIndex - this.maxVisible);
    } else if (kb.matches(keyData, "tui.select.pageDown")) {
      this.selectedIndex = Math.min(this.filteredSessions.length - 1, this.selectedIndex + this.maxVisible);
    } else if (kb.matches(keyData, "tui.select.confirm")) {
      const selected = this.filteredSessions[this.selectedIndex];
      if (selected && this.onSelect) {
        this.onSelect(selected.session.path);
      }
    } else if (kb.matches(keyData, "tui.select.cancel")) {
      if (this.onCancel) {
        this.onCancel();
      }
    } else {
      this.searchInput.handleInput(keyData);
      this.filterSessions(this.searchInput.getValue());
    }
  }
};
async function deleteSessionFile(sessionPath) {
  const trashArgs = sessionPath.startsWith("-") ? ["--", sessionPath] : [sessionPath];
  const trashResult = spawnSync4("trash", trashArgs, { encoding: "utf-8" });
  const getTrashErrorHint = () => {
    const parts = [];
    if (trashResult.error) {
      parts.push(trashResult.error.message);
    }
    const stderr = trashResult.stderr?.trim();
    if (stderr) {
      parts.push(stderr.split("\n")[0] ?? stderr);
    }
    if (parts.length === 0)
      return null;
    return `trash: ${parts.join(" \xB7 ").slice(0, 200)}`;
  };
  if (trashResult.status === 0 || !existsSync9(sessionPath)) {
    return { ok: true, method: "trash" };
  }
  try {
    await unlink(sessionPath);
    return { ok: true, method: "unlink" };
  } catch (err) {
    const unlinkError = err instanceof Error ? err.message : String(err);
    const trashErrorHint = getTrashErrorHint();
    const error = trashErrorHint ? `${unlinkError} (${trashErrorHint})` : unlinkError;
    return { ok: false, method: "unlink", error };
  }
}
var SessionSelectorComponent = class extends Container {
  handleInput(data) {
    if (this.mode === "rename") {
      const kb = getKeybindings();
      if (kb.matches(data, "tui.select.cancel")) {
        this.exitRenameMode();
        return;
      }
      this.renameInput.handleInput(data);
      return;
    }
    this.sessionList.handleInput(data);
  }
  canRename = true;
  sessionList;
  header;
  keybindings;
  scope = "current";
  sortMode = "threaded";
  nameFilter = "all";
  currentSessions = null;
  allSessions = null;
  currentSessionsLoader;
  allSessionsLoader;
  requestRender;
  renameSession;
  currentLoad = null;
  allLoad = null;
  mode = "list";
  renameInput = new Input();
  renameTargetPath = null;
  // Focusable implementation - propagate to sessionList for IME cursor positioning
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    this.sessionList.focused = value;
    this.renameInput.focused = value;
    if (value && this.mode === "rename") {
      this.renameInput.focused = true;
    }
  }
  buildBaseLayout(content, options) {
    this.clear();
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder((s) => theme_exports.theme.fg("accent", s)));
    this.addChild(new Spacer(1));
    if (options?.showHeader ?? true) {
      this.addChild(this.header);
      this.addChild(new Spacer(1));
    }
    this.addChild(content);
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder((s) => theme_exports.theme.fg("accent", s)));
  }
  constructor(currentSessionsLoader, allSessionsLoader, onSelect, onCancel, onExit, requestRender, options, currentSessionFilePath) {
    super();
    this.keybindings = options?.keybindings ?? KeybindingsManager2.create();
    this.currentSessionsLoader = currentSessionsLoader;
    this.allSessionsLoader = allSessionsLoader;
    this.requestRender = requestRender;
    this.header = new SessionSelectorHeader(this.scope, this.sortMode, this.nameFilter, this.requestRender);
    const renameSession = options?.renameSession;
    this.renameSession = renameSession;
    this.canRename = !!renameSession;
    this.header.setShowRenameHint(options?.showRenameHint ?? this.canRename);
    this.sessionList = new SessionList([], false, this.sortMode, this.nameFilter, this.keybindings, currentSessionFilePath);
    this.buildBaseLayout(this.sessionList);
    this.renameInput.onSubmit = (value) => {
      void this.confirmRename(value);
    };
    const clearStatusMessage = () => this.header.setStatusMessage(null);
    this.sessionList.onSelect = (sessionPath) => {
      clearStatusMessage();
      this.cancelLoads();
      onSelect(sessionPath);
    };
    this.sessionList.onCancel = () => {
      clearStatusMessage();
      this.cancelLoads();
      onCancel();
    };
    this.sessionList.onExit = () => {
      clearStatusMessage();
      this.cancelLoads();
      onExit();
    };
    this.sessionList.onToggleScope = () => this.toggleScope();
    this.sessionList.onToggleSort = () => this.toggleSortMode();
    this.sessionList.onToggleNameFilter = () => this.toggleNameFilter();
    this.sessionList.onRenameSession = (sessionPath) => {
      if (!renameSession)
        return;
      if (this.scope === "current" ? this.currentLoad : this.allLoad)
        return;
      const sessions = this.scope === "all" ? this.allSessions ?? [] : this.currentSessions ?? [];
      const session = sessions.find((s) => s.path === sessionPath);
      this.enterRenameMode(sessionPath, session?.name);
    };
    this.sessionList.onTogglePath = (showPath) => {
      this.header.setShowPath(showPath);
      this.requestRender();
    };
    this.sessionList.onDeleteConfirmationChange = (path4) => {
      this.header.setConfirmingDeletePath(path4);
      this.requestRender();
    };
    this.sessionList.onError = (msg) => {
      this.header.setStatusMessage({ type: "error", message: msg }, 3e3);
      this.requestRender();
    };
    this.sessionList.onDeleteSession = async (sessionPath) => {
      const result = await deleteSessionFile(sessionPath);
      if (result.ok) {
        if (this.currentSessions) {
          this.currentSessions = this.currentSessions.filter((s) => s.path !== sessionPath);
        }
        if (this.allSessions) {
          this.allSessions = this.allSessions.filter((s) => s.path !== sessionPath);
        }
        const sessions = this.scope === "all" ? this.allSessions ?? [] : this.currentSessions ?? [];
        const showCwd = this.scope === "all";
        this.sessionList.setSessions(sessions, showCwd);
        const msg = result.method === "trash" ? "Session moved to trash" : "Session deleted";
        this.header.setStatusMessage({ type: "info", message: msg }, 2e3);
        await this.refreshSessionsAfterMutation();
      } else {
        const errorMessage = result.error ?? "Unknown error";
        this.header.setStatusMessage({ type: "error", message: `Failed to delete: ${errorMessage}` }, 3e3);
      }
      this.requestRender();
    };
    void this.loadScope("current");
  }
  cancelLoads() {
    if (this.currentLoad) {
      this.currentLoad.abort();
      this.currentLoad = null;
      this.currentSessions = null;
    }
    if (this.allLoad) {
      this.allLoad.abort();
      this.allLoad = null;
      this.allSessions = null;
    }
  }
  enterRenameMode(sessionPath, currentName) {
    this.mode = "rename";
    this.renameTargetPath = sessionPath;
    this.renameInput.setValue(currentName ?? "");
    this.renameInput.focused = true;
    const panel = new Container();
    panel.addChild(new Text(theme_exports.theme.bold("Rename Session"), 1, 0));
    panel.addChild(new Spacer(1));
    panel.addChild(this.renameInput);
    panel.addChild(new Spacer(1));
    panel.addChild(new Text(theme_exports.theme.fg("muted", `${keyText("tui.select.confirm")} to save \xB7 ${keyText("tui.select.cancel")} to cancel`), 1, 0));
    this.buildBaseLayout(panel, { showHeader: false });
    this.requestRender();
  }
  exitRenameMode() {
    this.mode = "list";
    this.renameTargetPath = null;
    this.buildBaseLayout(this.sessionList);
    this.requestRender();
  }
  async confirmRename(value) {
    const next = value.trim();
    if (!next)
      return;
    const target = this.renameTargetPath;
    if (!target) {
      this.exitRenameMode();
      return;
    }
    const renameSession = this.renameSession;
    if (!renameSession) {
      this.exitRenameMode();
      return;
    }
    try {
      await renameSession(target, next);
      await this.refreshSessionsAfterMutation();
    } finally {
      this.exitRenameMode();
    }
  }
  async loadScope(scope) {
    if (scope === "current" ? this.currentLoad : this.allLoad)
      return;
    const showCwd = scope === "all";
    const controller = new AbortController();
    if (scope === "current") {
      this.currentLoad = controller;
    } else {
      this.allLoad = controller;
    }
    this.header.setScope(scope);
    this.header.setLoading(true);
    this.requestRender();
    const isActive = () => (scope === "current" ? this.currentLoad : this.allLoad) === controller;
    const onProgress = (loaded, total, partialSessions) => {
      if (!isActive())
        return;
      if (partialSessions) {
        const sessions = [...partialSessions];
        if (scope === "current") {
          this.currentSessions = sessions;
        } else {
          this.allSessions = sessions;
        }
        if (scope === this.scope)
          this.sessionList.setSessions(sessions, showCwd);
      }
      if (scope !== this.scope)
        return;
      this.header.setProgress(loaded, total);
      this.requestRender();
    };
    try {
      const sessions = await (scope === "current" ? this.currentSessionsLoader(onProgress, controller.signal) : this.allSessionsLoader(onProgress, controller.signal));
      if (!isActive())
        return;
      if (scope === "current") {
        this.currentSessions = sessions;
        this.currentLoad = null;
      } else {
        this.allSessions = sessions;
        this.allLoad = null;
      }
      if (scope !== this.scope)
        return;
      this.header.setLoading(false);
      this.sessionList.setSessions(sessions, showCwd);
      this.requestRender();
    } catch (err) {
      if (!isActive())
        return;
      if (scope === "current") {
        this.currentLoad = null;
        this.currentSessions = null;
      } else {
        this.allLoad = null;
        this.allSessions = null;
      }
      if (scope !== this.scope)
        return;
      const message = err instanceof Error ? err.message : String(err);
      this.header.setLoading(false);
      this.header.setStatusMessage({ type: "error", message: `Failed to load sessions: ${message}` }, 4e3);
      this.sessionList.setSessions([], showCwd);
      this.requestRender();
    }
  }
  toggleSortMode() {
    this.sortMode = this.sortMode === "threaded" ? "recent" : this.sortMode === "recent" ? "relevance" : "threaded";
    this.header.setSortMode(this.sortMode);
    this.sessionList.setSortMode(this.sortMode);
    this.requestRender();
  }
  toggleNameFilter() {
    this.nameFilter = this.nameFilter === "all" ? "named" : "all";
    this.header.setNameFilter(this.nameFilter);
    this.sessionList.setNameFilter(this.nameFilter);
    this.requestRender();
  }
  async refreshSessionsAfterMutation() {
    this.cancelLoads();
    this.currentSessions = null;
    this.allSessions = null;
    await this.loadScope(this.scope);
  }
  toggleScope() {
    this.scope = this.scope === "current" ? "all" : "current";
    const sessions = this.scope === "current" ? this.currentSessions : this.allSessions;
    const loading = (this.scope === "current" ? this.currentLoad : this.allLoad) !== null;
    this.header.setScope(this.scope);
    this.header.setLoading(loading);
    this.sessionList.setSessions(sessions ?? [], this.scope === "all");
    this.requestRender();
    if (sessions === null && !loading)
      void this.loadScope(this.scope);
  }
  getSessionList() {
    return this.sessionList;
  }
};

// target/vendor-pi/dist/modes/interactive/components/settings-selector.js
import { getSupportedThinkingLevels } from "@earendil-works/pi-ai";

// target/vendor-pi/dist/modes/interactive/components/settings-submenu.js
var SUBMENU_SELECT_LIST_LAYOUT = {
  minPrimaryColumnWidth: 12,
  maxPrimaryColumnWidth: 32
};
var SelectSubmenu = class extends Container {
  selectList;
  listChildIndex;
  allOptions;
  listLayout;
  searchInput;
  onSelectCb;
  onCancelCb;
  onSelectionChangeCb;
  constructor(title, description, options, currentValue, onSelect, onCancel, onSelectionChange, submenuOptions) {
    super();
    this.allOptions = options;
    this.listLayout = submenuOptions?.layout ?? SUBMENU_SELECT_LIST_LAYOUT;
    this.onSelectCb = onSelect;
    this.onCancelCb = onCancel;
    this.onSelectionChangeCb = onSelectionChange;
    this.addChild(new Text(theme_exports.theme.bold(theme_exports.theme.fg("accent", title)), 0, 0));
    if (description) {
      this.addChild(new Spacer(1));
      this.addChild(new Text(theme_exports.theme.fg("muted", description), 0, 0));
    }
    if (submenuOptions?.searchable) {
      this.addChild(new Spacer(1));
      this.searchInput = new Input();
      this.searchInput.onSubmit = () => {
        this.selectList.handleInput("\r");
      };
      this.addChild(this.searchInput);
    }
    this.addChild(new Spacer(1));
    this.selectList = this.buildSelectList(options, currentValue);
    this.listChildIndex = this.children.length;
    this.addChild(this.selectList);
    this.addChild(new Spacer(1));
    const hint = submenuOptions?.searchable ? "  Type to filter \xB7 Enter to select \xB7 Esc to go back" : "  Enter to select \xB7 Esc to go back";
    this.addChild(new Text(theme_exports.theme.fg("dim", hint), 0, 0));
  }
  buildSelectList(options, preselect) {
    const list = new SelectList(options, Math.min(options.length, 10), (0, theme_exports.getSelectListTheme)(), this.listLayout);
    const idx = options.findIndex((o) => o.value === preselect);
    if (idx !== -1)
      list.setSelectedIndex(idx);
    list.onSelect = (item) => this.onSelectCb(item.value);
    list.onCancel = this.onCancelCb;
    if (this.onSelectionChangeCb) {
      const cb = this.onSelectionChangeCb;
      list.onSelectionChange = (item) => cb(item.value);
    }
    return list;
  }
  applyFilter(query) {
    const filtered = query ? fuzzyFilter(this.allOptions, query, (item) => `${item.label} ${item.description ?? ""}`) : this.allOptions;
    const newList = this.buildSelectList(filtered, "");
    this.children[this.listChildIndex] = newList;
    this.selectList = newList;
  }
  handleInput(data) {
    if (this.searchInput) {
      const kb = getKeybindings();
      const isNav = kb.matches(data, "tui.select.up") || kb.matches(data, "tui.select.down") || kb.matches(data, "tui.select.confirm") || kb.matches(data, "tui.select.cancel");
      if (isNav) {
        this.selectList.handleInput(data);
      } else {
        this.searchInput.handleInput(data);
        this.applyFilter(this.searchInput.getValue());
      }
    } else {
      this.selectList.handleInput(data);
    }
  }
};
var SteppedSubmenu = class extends Container {
  steps;
  onComplete;
  onCancel;
  opts;
  activeComponent;
  context;
  constructor(steps, onComplete, onCancel, opts = {}) {
    super();
    this.steps = steps;
    this.onComplete = onComplete;
    this.onCancel = onCancel;
    this.opts = opts;
    this.context = { ...opts.initialContext ?? {} };
    this.activeComponent = this.buildStep(opts.startAtStep ?? 0);
  }
  buildStep(stepIndex) {
    const step = this.steps[stepIndex];
    const total = this.steps.length;
    const stepLabel = total > 1 ? `Step ${stepIndex + 1}/${total} \xB7 ` : "";
    const title = typeof step.title === "function" ? step.title(this.context) : step.title;
    const desc = typeof step.description === "function" ? step.description(this.context) : step.description;
    const items = step.options(this.context);
    const preselect = step.preselect?.(this.context) ?? "";
    return new SelectSubmenu(title, `${stepLabel}${desc}`, items, preselect, (value) => {
      this.context[step.key] = value;
      if (stepIndex < total - 1) {
        this.activeComponent = this.buildStep(stepIndex + 1);
      } else {
        this.onComplete({ ...this.context });
        if (this.opts.loop) {
          this.context = {};
          this.activeComponent = this.buildStep(0);
        } else {
          this.onCancel();
        }
      }
    }, () => {
      if (stepIndex > 0) {
        delete this.context[step.key];
        this.activeComponent = this.buildStep(stepIndex - 1);
      } else {
        this.onCancel();
      }
    }, void 0, step.searchable || step.layout ? { searchable: step.searchable, layout: step.layout } : void 0);
  }
  render(width) {
    return this.activeComponent.render(width);
  }
  handleInput(data) {
    this.activeComponent.handleInput?.(data);
  }
  invalidate() {
    this.activeComponent.invalidate?.();
  }
};

// target/vendor-pi/dist/modes/interactive/components/settings-selector.js
var MODEL_PICKER_LAYOUT = { minPrimaryColumnWidth: 12, maxPrimaryColumnWidth: 46 };
var THINKING_DESCRIPTIONS = {
  off: "No reasoning",
  minimal: "Very brief reasoning (~1k tokens)",
  low: "Light reasoning (~2k tokens)",
  medium: "Moderate reasoning (~8k tokens)",
  high: "Deep reasoning (~16k tokens)",
  xhigh: "Extra-high reasoning (~32k tokens)",
  max: "Maximum reasoning"
};
var DEFAULT_PROJECT_TRUST_LABELS = {
  ask: "Ask",
  always: "Always trust",
  never: "Never trust"
};
var DEFAULT_PROJECT_TRUST_BY_LABEL = new Map(Object.entries(DEFAULT_PROJECT_TRUST_LABELS).map(([value, label]) => [label, value]));
var WarningSettingsSubmenu = class extends Container {
  settingsList;
  state;
  constructor(warnings, onChange, onCancel) {
    super();
    this.state = { ...warnings };
    const items = [
      {
        id: "anthropic-extra-usage",
        label: "Anthropic extra usage",
        description: "Warn when Anthropic subscription auth may use paid extra usage",
        currentValue: this.state.anthropicExtraUsage ?? true ? "true" : "false",
        values: ["true", "false"]
      }
    ];
    this.settingsList = new SettingsList(items, Math.min(items.length, 10), (0, theme_exports.getSettingsListTheme)(), (id, newValue) => {
      switch (id) {
        case "anthropic-extra-usage":
          this.state = { ...this.state, anthropicExtraUsage: newValue === "true" };
          onChange({ ...this.state });
          break;
      }
    }, onCancel);
    this.addChild(this.settingsList);
  }
  handleInput(data) {
    this.settingsList.handleInput(data);
  }
};
var CLEAR_OVERRIDE_VALUE = "__clear__";
function modelSettingKey(model) {
  return `${model.provider}/${model.id}`;
}
function modelDisplayLabel(model) {
  return `${model.id} [${model.provider}]`;
}
function modelThinkingOverridesSummary(overrides) {
  const count = Object.keys(overrides).length;
  if (count === 0)
    return "none";
  return `${count} configured`;
}
function modelItemLabel(model) {
  return `${model.id} ${theme_exports.theme.fg("muted", `[${model.provider}]`)}`;
}
function themeItems(availableThemes, currentTheme) {
  return availableThemes.map((name) => ({
    value: name,
    label: `${name === currentTheme ? "\u2713 " : "  "}${name}`,
    ...name === theme_exports.SYSTEM_THEME_NAME ? { description: "Theme created from your terminal's colors" } : {}
  }));
}
var AUTOMATIC_THEME_VALUE = "/";
function singleModeThemeItems(availableThemes, currentTheme) {
  const items = themeItems(availableThemes, currentTheme);
  const systemIndex = items.findIndex((item) => item.value === theme_exports.SYSTEM_THEME_NAME);
  const system = systemIndex === -1 ? [] : items.splice(systemIndex, 1);
  return [
    ...system,
    {
      value: AUTOMATIC_THEME_VALUE,
      label: "  automatic",
      description: "Use separate themes for light and dark terminal appearance"
    },
    ...items
  ];
}
function preferredTheme(availableThemes, preferred, fallback) {
  if (preferred && availableThemes.includes(preferred))
    return preferred;
  if (availableThemes.includes(fallback))
    return fallback;
  return availableThemes[0] ?? fallback;
}
function defaultAutomaticThemes(currentThemeSetting, availableThemes) {
  const autoTheme = (0, theme_exports.parseAutoThemeSetting)(currentThemeSetting);
  if (autoTheme)
    return autoTheme;
  const currentFixedTheme = currentThemeSetting.includes("/") ? void 0 : currentThemeSetting;
  const themeName = preferredTheme(availableThemes, currentFixedTheme, theme_exports.SYSTEM_THEME_NAME);
  return { lightTheme: themeName, darkTheme: themeName };
}
var ThemeSubmenu = class extends Container {
  inputComponent;
  callbacks;
  availableThemes;
  terminalTheme;
  onDone;
  originalThemeSetting;
  mode;
  singleTheme;
  lightTheme;
  darkTheme;
  constructor(currentThemeSetting, terminalTheme, availableThemes, callbacks, onDone) {
    super();
    this.callbacks = callbacks;
    this.availableThemes = availableThemes;
    this.terminalTheme = terminalTheme;
    this.onDone = onDone;
    this.originalThemeSetting = currentThemeSetting;
    const autoTheme = (0, theme_exports.parseAutoThemeSetting)(currentThemeSetting);
    const automaticThemes = defaultAutomaticThemes(currentThemeSetting, availableThemes);
    const fixedTheme = autoTheme || currentThemeSetting.includes("/") ? void 0 : currentThemeSetting;
    this.mode = autoTheme ? "automatic" : "single";
    this.lightTheme = automaticThemes.lightTheme;
    this.darkTheme = automaticThemes.darkTheme;
    this.singleTheme = preferredTheme(availableThemes, fixedTheme ?? (autoTheme ? this.getActiveAutomaticTheme() : void 0), theme_exports.SYSTEM_THEME_NAME);
    if (this.mode === "automatic") {
      this.showAutomaticMenu();
    } else {
      this.showSingleMenu();
    }
  }
  handleInput(data) {
    this.inputComponent?.handleInput?.(data);
  }
  setContent(renderComponent, inputComponent = renderComponent) {
    this.clear();
    this.addChild(renderComponent);
    this.inputComponent = inputComponent;
  }
  showSingleMenu() {
    this.mode = "single";
    const menu = new SelectSubmenu("Theme", "Select a theme, or choose automatic to follow terminal appearance.", singleModeThemeItems(this.availableThemes, this.singleTheme), this.singleTheme, (value) => {
      if (value === AUTOMATIC_THEME_VALUE) {
        this.mode = "automatic";
        this.callbacks.onThemePreview?.(this.getThemeSetting());
        this.showAutomaticMenu();
        return;
      }
      this.singleTheme = value;
      this.apply(value);
    }, () => this.cancel(), (value) => {
      this.callbacks.onThemePreview?.(value === AUTOMATIC_THEME_VALUE ? this.getAutomaticThemeSetting() : value);
    });
    this.setContent(menu);
  }
  showAutomaticMenu() {
    this.mode = "automatic";
    const content = new Container();
    content.addChild(new Text(theme_exports.theme.bold(theme_exports.theme.fg("accent", "Automatic Theme")), 0, 0));
    content.addChild(new Spacer(1));
    content.addChild(new Text(theme_exports.theme.fg("muted", "Choose themes for terminal light and dark appearance."), 0, 0));
    content.addChild(new Text(theme_exports.theme.fg("muted", "Light/dark detection requires terminal support."), 0, 0));
    content.addChild(new Spacer(1));
    const items = [
      {
        id: "light-theme",
        label: "Light theme",
        description: "Theme to use in automatic mode when the terminal is light",
        currentValue: this.lightTheme,
        submenu: (currentValue, done) => this.createThemeSelect("Light Theme", "Select the theme to use for light terminal appearance", currentValue, done, (value) => {
          this.lightTheme = value;
          this.callbacks.onThemePreview?.(this.getThemeSetting());
          done(value);
        })
      },
      {
        id: "dark-theme",
        label: "Dark theme",
        description: "Theme to use in automatic mode when the terminal is dark",
        currentValue: this.darkTheme,
        submenu: (currentValue, done) => this.createThemeSelect("Dark Theme", "Select the theme to use for dark terminal appearance", currentValue, done, (value) => {
          this.darkTheme = value;
          this.callbacks.onThemePreview?.(this.getThemeSetting());
          done(value);
        })
      },
      {
        id: "apply",
        label: "Apply",
        description: "Save and go back",
        currentValue: "save and go back",
        values: ["save and go back"]
      },
      {
        id: "single-mode",
        label: "Change mode",
        description: "Switch to one theme for light and dark",
        currentValue: "switch to single theme",
        values: ["switch to single theme"]
      }
    ];
    const settingsList = new SettingsList(items, Math.min(items.length, 10), (0, theme_exports.getSettingsListTheme)(), (id) => {
      switch (id) {
        case "single-mode":
          this.mode = "single";
          this.singleTheme = this.getActiveAutomaticTheme();
          this.callbacks.onThemePreview?.(this.singleTheme);
          this.showSingleMenu();
          break;
        case "apply":
          this.apply(this.getAutomaticThemeSetting());
          break;
      }
    }, () => this.cancel());
    content.addChild(settingsList);
    this.setContent(content, settingsList);
  }
  createThemeSelect(title, description, currentValue, done, onSelect) {
    return new SelectSubmenu(title, description, themeItems(this.availableThemes, currentValue), currentValue, onSelect, () => {
      this.callbacks.onThemePreview?.(this.getThemeSetting());
      done();
    }, (value) => this.callbacks.onThemePreview?.(value));
  }
  getThemeSetting() {
    return this.mode === "automatic" ? this.getAutomaticThemeSetting() : this.singleTheme;
  }
  getActiveAutomaticTheme() {
    return this.terminalTheme === "light" ? this.lightTheme : this.darkTheme;
  }
  getAutomaticThemeSetting() {
    return `${this.lightTheme}/${this.darkTheme}`;
  }
  apply(themeSetting) {
    this.onDone(themeSetting);
  }
  cancel() {
    this.callbacks.onThemePreview?.(this.originalThemeSetting);
    this.onDone();
  }
};
var SettingsSelectorComponent = class extends Container {
  settingsList;
  constructor(config, callbacks) {
    super();
    const supportsImages = getCapabilities().images;
    const followUpKey = keyDisplayText("app.message.followUp");
    const cycleThinkingKey = keyDisplayText("app.thinking.cycle");
    let currentWarnings = { ...config.warnings };
    const currentModelThinkingLevels = { ...config.modelThinkingLevels };
    const defaultModelByValue = new Map(config.availableDefaultModels.map((model) => [modelSettingKey(model), model]));
    const currentDefaultModelKey = defaultModelByValue.has(config.defaultModel) ? config.defaultModel : void 0;
    const currentModelKey = config.currentModel ? modelSettingKey(config.currentModel) : void 0;
    const items = [
      {
        id: "autocompact",
        label: "Auto-compact",
        description: "Automatically compact context when it gets too large",
        currentValue: config.autoCompact ? "true" : "false",
        values: ["true", "false"]
      },
      {
        id: "steering-mode",
        label: "Steering mode",
        description: "Enter while streaming queues steering messages. 'one-at-a-time': deliver one, wait for response. 'all': deliver all at once.",
        currentValue: config.steeringMode,
        values: ["one-at-a-time", "all"]
      },
      {
        id: "follow-up-mode",
        label: "Follow-up mode",
        description: `${followUpKey} queues follow-up messages until agent stops. 'one-at-a-time': deliver one, wait for response. 'all': deliver all at once.`,
        currentValue: config.followUpMode,
        values: ["one-at-a-time", "all"]
      },
      {
        id: "transport",
        label: "Transport",
        description: "Preferred transport for providers that support multiple transports",
        currentValue: config.transport,
        values: ["sse", "websocket", "websocket-cached", "auto"]
      },
      {
        id: "http-idle-timeout",
        label: "HTTP idle timeout",
        description: "Maximum idle gap while waiting for HTTP headers or body chunks. Disable for local models that pause longer than five minutes.",
        currentValue: formatHttpIdleTimeoutMs(config.httpIdleTimeoutMs),
        values: HTTP_IDLE_TIMEOUT_CHOICES.map((choice) => choice.label)
      },
      {
        id: "cache-warming-mode",
        label: "Cache warming",
        description: "off; streaming while the agent runs; idle also between runs while continuation stays profitable",
        currentValue: config.cacheWarmingMode,
        values: [...CACHE_WARMING_MODES]
      },
      {
        id: "hide-thinking",
        label: "Hide thinking",
        description: "Hide thinking blocks in assistant responses",
        currentValue: config.hideThinkingBlock ? "true" : "false",
        values: ["true", "false"]
      },
      {
        id: "mermaid-rendering",
        label: "Mermaid diagrams",
        description: "Render Mermaid code blocks as Unicode diagrams",
        currentValue: config.mermaidRenderingMode,
        values: ["off", "final", "streaming"]
      },
      {
        id: "cache-miss-notices",
        label: "Cache miss notices",
        description: "Show transcript notices for cache costs and provider recovery diagnostics",
        currentValue: config.showCacheMissNotices ? "true" : "false",
        values: ["true", "false"]
      },
      {
        id: "collapse-changelog",
        label: "Collapse changelog",
        description: "Show condensed changelog after updates",
        currentValue: config.collapseChangelog ? "true" : "false",
        values: ["true", "false"]
      },
      {
        id: "quiet-startup",
        label: "Quiet startup",
        description: "Disable verbose printing at startup (header: keep only the startup header)",
        currentValue: String(config.quietStartup),
        values: ["true", "header", "false"]
      },
      {
        id: "install-telemetry",
        label: "Install telemetry",
        description: "Send an anonymous version/update ping after changelog-detected updates",
        currentValue: config.enableInstallTelemetry ? "true" : "false",
        values: ["true", "false"]
      },
      {
        id: "default-project-trust",
        label: "Default project trust",
        description: "Fallback behavior when no extension or saved trust decision decides project trust",
        currentValue: DEFAULT_PROJECT_TRUST_LABELS[config.defaultProjectTrust],
        values: Object.values(DEFAULT_PROJECT_TRUST_LABELS)
      },
      {
        id: "double-escape-action",
        label: "Double-escape action",
        description: "Action when pressing Escape twice with empty editor",
        currentValue: config.doubleEscapeAction,
        values: ["tree", "fork", "none"]
      },
      {
        id: "tree-filter-mode",
        label: "Tree filter mode",
        description: "Default filter when opening /tree",
        currentValue: config.treeFilterMode,
        values: ["default", "no-tools", "user-only", "labeled-only", "all"]
      },
      {
        id: "warnings",
        label: "Warnings",
        description: "Enable or disable individual warnings",
        currentValue: "configure",
        submenu: (_currentValue, done) => new WarningSettingsSubmenu(currentWarnings, (warnings) => {
          currentWarnings = warnings;
          callbacks.onWarningsChange(warnings);
        }, () => done())
      },
      {
        id: "model-thinking",
        label: "Default thinking level per model",
        description: `Override the default thinking level for specific models. ${cycleThinkingKey} cycles in-session.`,
        currentValue: modelThinkingOverridesSummary(currentModelThinkingLevels),
        submenu: (_currentValue, done) => {
          const steps = [
            {
              key: "model",
              title: "Per-Model Thinking Level",
              description: "Select a model to configure",
              options: () => {
                const sorted = [...config.availableDefaultModels].sort((a, b) => {
                  const aKey = modelSettingKey(a);
                  const bKey = modelSettingKey(b);
                  if (aKey === currentModelKey)
                    return -1;
                  if (bKey === currentModelKey)
                    return 1;
                  if (aKey === currentDefaultModelKey)
                    return -1;
                  if (bKey === currentDefaultModelKey)
                    return 1;
                  return a.provider.localeCompare(b.provider);
                });
                const items2 = sorted.map((model) => {
                  const key = modelSettingKey(model);
                  const override = currentModelThinkingLevels[key];
                  return {
                    value: key,
                    label: modelItemLabel(model),
                    description: override ?? void 0
                  };
                });
                if (items2.length === 0) {
                  items2.push({
                    value: "__none__",
                    label: "No models available",
                    description: "Log in to a provider or configure an API key first"
                  });
                }
                return items2;
              },
              preselect: () => currentModelKey ?? currentDefaultModelKey,
              searchable: true,
              layout: MODEL_PICKER_LAYOUT
            },
            {
              key: "level",
              title: (ctx) => {
                const m = defaultModelByValue.get(ctx.model);
                return `Thinking Level for ${m ? modelDisplayLabel(m) : ctx.model}`;
              },
              description: "Select default thinking level for this model",
              options: (ctx) => {
                const model = defaultModelByValue.get(ctx.model);
                if (!model)
                  return [];
                const levels = model.reasoning ? getSupportedThinkingLevels(model) : ["off"];
                const activeLevel = currentModelThinkingLevels[ctx.model];
                const items2 = levels.map((level) => ({
                  value: level,
                  label: `${level === activeLevel ? "\u2713 " : "  "}${level}`,
                  description: THINKING_DESCRIPTIONS[level]
                }));
                if (currentModelThinkingLevels[ctx.model] !== void 0) {
                  items2.push({
                    value: CLEAR_OVERRIDE_VALUE,
                    label: "  (clear override)",
                    description: `Revert to global default (${config.thinkingLevel})`
                  });
                }
                return items2;
              },
              preselect: (ctx) => currentModelThinkingLevels[ctx.model]
            }
          ];
          const summary = () => modelThinkingOverridesSummary(currentModelThinkingLevels);
          return new SteppedSubmenu(steps, (selections) => {
            const model = defaultModelByValue.get(selections.model);
            if (!model)
              return;
            if (selections.level === CLEAR_OVERRIDE_VALUE) {
              callbacks.onModelThinkingLevelRemove(model.provider, model.id);
              delete currentModelThinkingLevels[selections.model];
            } else {
              callbacks.onModelThinkingLevelChange(model.provider, model.id, selections.level);
              currentModelThinkingLevels[selections.model] = selections.level;
            }
          }, () => {
            done(summary());
          }, { loop: true });
        }
      },
      {
        id: "tui-mode",
        label: "TUI mode",
        description: "Interface layout; regular mode uses the terminal's normal scrollback",
        currentValue: config.tuiMode,
        values: ["regular", "fullscreen"]
      },
      {
        id: "fullscreen-exit-output",
        label: "Fullscreen exit output",
        description: "Print the transcript or only a session resume hint when exiting fullscreen mode",
        currentValue: config.fullscreenExitOutput,
        values: ["transcript", "resume-hint"]
      },
      {
        id: "fullscreen-scrollbar",
        label: "Fullscreen scrollbar",
        description: "Scrollbar behavior in fullscreen mode; has no effect in regular mode",
        currentValue: config.fullscreenScrollbar,
        values: ["auto", "always", "hidden"]
      },
      {
        id: "fullscreen-copy-on-select",
        label: "Fullscreen copy on select",
        description: "Automatically copy selected text in fullscreen mode; disable to copy selections with Ctrl+X",
        currentValue: config.fullscreenCopyOnSelect ? "true" : "false",
        values: ["true", "false"]
      },
      {
        id: "fullscreen-wheel-scroll-lines",
        label: "Fullscreen wheel scrolling",
        description: "Lines per mouse-wheel event in fullscreen mode; 'auto' speeds up fast wheel spins where the terminal does not",
        currentValue: String(config.fullscreenWheelScrollLines),
        values: [
          "auto",
          ...[.../* @__PURE__ */ new Set([1, 2, 3, 5, 10, config.fullscreenWheelScrollLines])].filter((lines) => lines !== "auto").sort((a, b) => a - b).map(String)
        ]
      },
      {
        id: "theme",
        label: "Theme",
        description: "Color theme for the interface",
        currentValue: config.currentTheme,
        submenu: (currentValue, done) => new ThemeSubmenu(currentValue, config.terminalTheme, config.availableThemes, callbacks, done)
      }
    ];
    if (supportsImages) {
      items.splice(1, 0, {
        id: "show-images",
        label: "Show images",
        description: "Render images inline in terminal",
        currentValue: config.showImages ? "true" : "false",
        values: ["true", "false"]
      });
      items.splice(2, 0, {
        id: "image-width-cells",
        label: "Image width",
        description: "Preferred inline image width in terminal cells",
        currentValue: String(config.imageWidthCells),
        values: ["60", "80", "120"]
      });
    }
    items.splice(supportsImages ? 3 : 1, 0, {
      id: "auto-resize-images",
      label: "Auto-resize images",
      description: "Resize large images to 2000x2000 max for better model compatibility",
      currentValue: config.autoResizeImages ? "true" : "false",
      values: ["true", "false"]
    });
    const autoResizeIndex = items.findIndex((item) => item.id === "auto-resize-images");
    items.splice(autoResizeIndex + 1, 0, {
      id: "block-images",
      label: "Block images",
      description: "Prevent images from being sent to LLM providers",
      currentValue: config.blockImages ? "true" : "false",
      values: ["true", "false"]
    });
    const blockImagesIndex = items.findIndex((item) => item.id === "block-images");
    items.splice(blockImagesIndex + 1, 0, {
      id: "skill-commands",
      label: "Skill commands",
      description: "Register skills as /skill:name commands",
      currentValue: config.enableSkillCommands ? "true" : "false",
      values: ["true", "false"]
    });
    const skillCommandsIndex = items.findIndex((item) => item.id === "skill-commands");
    items.splice(skillCommandsIndex + 1, 0, {
      id: "show-hardware-cursor",
      label: "Show hardware cursor",
      description: "Show the terminal cursor while still positioning it for IME support",
      currentValue: config.showHardwareCursor ? "true" : "false",
      values: ["true", "false"]
    });
    const hardwareCursorIndex = items.findIndex((item) => item.id === "show-hardware-cursor");
    items.splice(hardwareCursorIndex + 1, 0, {
      id: "editor-padding",
      label: "Editor padding",
      description: "Horizontal padding for input editor (0-3)",
      currentValue: String(config.editorPaddingX),
      values: ["0", "1", "2", "3"]
    });
    const editorPaddingIndex = items.findIndex((item) => item.id === "editor-padding");
    items.splice(editorPaddingIndex + 1, 0, {
      id: "output-padding",
      label: "Output padding",
      description: "Horizontal padding for user messages, assistant messages, and thinking",
      currentValue: String(config.outputPad),
      values: ["0", "1"]
    });
    const outputPaddingIndex = items.findIndex((item) => item.id === "output-padding");
    items.splice(outputPaddingIndex + 1, 0, {
      id: "autocomplete-max-visible",
      label: "Autocomplete max items",
      description: "Max visible items in autocomplete dropdown (3-20)",
      currentValue: String(config.autocompleteMaxVisible),
      values: ["3", "5", "7", "10", "15", "20"]
    });
    const autocompleteIndex = items.findIndex((item) => item.id === "autocomplete-max-visible");
    items.splice(autocompleteIndex + 1, 0, {
      id: "clear-on-shrink",
      label: "Clear on shrink",
      description: "Clear empty rows when content shrinks (may cause flicker)",
      currentValue: config.clearOnShrink ? "true" : "false",
      values: ["true", "false"]
    });
    const clearOnShrinkIndex = items.findIndex((item) => item.id === "clear-on-shrink");
    items.splice(clearOnShrinkIndex + 1, 0, {
      id: "terminal-progress",
      label: "Terminal progress",
      description: "Show OSC 9;4 progress indicators in the terminal tab bar",
      currentValue: config.showTerminalProgress ? "true" : "false",
      values: ["true", "false"]
    });
    this.addChild(new DynamicBorder());
    this.settingsList = new SettingsList(items, 10, (0, theme_exports.getSettingsListTheme)(), (id, newValue) => {
      switch (id) {
        case "autocompact":
          callbacks.onAutoCompactChange(newValue === "true");
          break;
        case "show-images":
          callbacks.onShowImagesChange(newValue === "true");
          break;
        case "image-width-cells":
          callbacks.onImageWidthCellsChange(parseInt(newValue, 10));
          break;
        case "auto-resize-images":
          callbacks.onAutoResizeImagesChange(newValue === "true");
          break;
        case "block-images":
          callbacks.onBlockImagesChange(newValue === "true");
          break;
        case "skill-commands":
          callbacks.onEnableSkillCommandsChange(newValue === "true");
          break;
        case "steering-mode":
          callbacks.onSteeringModeChange(newValue);
          break;
        case "follow-up-mode":
          callbacks.onFollowUpModeChange(newValue);
          break;
        case "transport":
          callbacks.onTransportChange(newValue);
          break;
        case "http-idle-timeout": {
          const choice = HTTP_IDLE_TIMEOUT_CHOICES.find((item) => item.label === newValue);
          if (choice) {
            callbacks.onHttpIdleTimeoutMsChange(choice.timeoutMs);
          }
          break;
        }
        case "cache-warming-mode":
          callbacks.onCacheWarmingModeChange(newValue);
          break;
        case "hide-thinking":
          callbacks.onHideThinkingBlockChange(newValue === "true");
          break;
        case "mermaid-rendering":
          callbacks.onMermaidRenderingModeChange(newValue);
          break;
        case "cache-miss-notices":
          callbacks.onShowCacheMissNoticesChange(newValue === "true");
          break;
        case "collapse-changelog":
          callbacks.onCollapseChangelogChange(newValue === "true");
          break;
        case "quiet-startup":
          callbacks.onQuietStartupChange(newValue === "header" ? "header" : newValue === "true");
          break;
        case "install-telemetry":
          callbacks.onEnableInstallTelemetryChange(newValue === "true");
          break;
        case "default-project-trust": {
          const defaultProjectTrust = DEFAULT_PROJECT_TRUST_BY_LABEL.get(newValue);
          if (defaultProjectTrust) {
            callbacks.onDefaultProjectTrustChange(defaultProjectTrust);
          }
          break;
        }
        case "double-escape-action":
          callbacks.onDoubleEscapeActionChange(newValue);
          break;
        case "tree-filter-mode":
          callbacks.onTreeFilterModeChange(newValue);
          break;
        case "show-hardware-cursor":
          callbacks.onShowHardwareCursorChange(newValue === "true");
          break;
        case "editor-padding":
          callbacks.onEditorPaddingXChange(parseInt(newValue, 10));
          break;
        case "output-padding":
          callbacks.onOutputPadChange(newValue === "0" ? 0 : 1);
          break;
        case "autocomplete-max-visible":
          callbacks.onAutocompleteMaxVisibleChange(parseInt(newValue, 10));
          break;
        case "clear-on-shrink":
          callbacks.onClearOnShrinkChange(newValue === "true");
          break;
        case "terminal-progress":
          callbacks.onShowTerminalProgressChange(newValue === "true");
          break;
        case "tui-mode":
          callbacks.onTuiModeChange(newValue);
          break;
        case "fullscreen-exit-output":
          callbacks.onFullscreenExitOutputChange(newValue);
          break;
        case "fullscreen-scrollbar":
          callbacks.onFullscreenScrollbarChange(newValue);
          break;
        case "fullscreen-copy-on-select":
          callbacks.onFullscreenCopyOnSelectChange(newValue === "true");
          break;
        case "fullscreen-wheel-scroll-lines":
          callbacks.onFullscreenWheelScrollLinesChange(newValue === "auto" ? "auto" : parseInt(newValue, 10));
          break;
        case "theme":
          callbacks.onThemeChange(newValue);
          break;
      }
    }, callbacks.onCancel, { enableSearch: true });
    this.addChild(this.settingsList);
    this.addChild(new DynamicBorder());
  }
  getSettingsList() {
    return this.settingsList;
  }
};

// target/vendor-pi/dist/modes/interactive/components/show-images-selector.js
var SHOW_IMAGES_SELECT_LIST_LAYOUT = {
  minPrimaryColumnWidth: 12,
  maxPrimaryColumnWidth: 32
};
var ShowImagesSelectorComponent = class extends Container {
  selectList;
  constructor(currentValue, onSelect, onCancel) {
    super();
    const items = [
      { value: "yes", label: "Yes", description: "Show images inline in terminal" },
      { value: "no", label: "No", description: "Show text placeholder instead" }
    ];
    this.addChild(new DynamicBorder());
    this.selectList = new SelectList(items, 5, (0, theme_exports.getSelectListTheme)(), SHOW_IMAGES_SELECT_LIST_LAYOUT);
    this.selectList.setSelectedIndex(currentValue ? 0 : 1);
    this.selectList.onSelect = (item) => {
      onSelect(item.value === "yes");
    };
    this.selectList.onCancel = () => {
      onCancel();
    };
    this.addChild(this.selectList);
    this.addChild(new DynamicBorder());
  }
  getSelectList() {
    return this.selectList;
  }
};

// target/vendor-pi/dist/modes/interactive/components/skill-invocation-message.js
var SkillInvocationMessageComponent = class extends Box {
  expanded = false;
  skillBlock;
  markdownTheme;
  constructor(skillBlock, markdownTheme = (0, theme_exports.getMarkdownTheme)()) {
    super(1, 1, (t) => theme_exports.theme.bg("customMessageBg", t));
    this.skillBlock = skillBlock;
    this.markdownTheme = markdownTheme;
    this.updateDisplay();
  }
  setExpanded(expanded) {
    this.expanded = expanded;
    this.updateDisplay();
  }
  invalidate() {
    super.invalidate();
    this.updateDisplay();
  }
  updateDisplay() {
    this.clear();
    const content = new Container();
    if (this.expanded) {
      const label = theme_exports.theme.fg("customMessageLabel", `\x1B[1m[skill]\x1B[22m`);
      content.addChild(new Text(label, 0, 0));
      const header = `**${this.skillBlock.name}**

`;
      content.addChild(new Markdown(header + this.skillBlock.content, 0, 0, this.markdownTheme, {
        color: (text) => theme_exports.theme.fg("customMessageText", text)
      }));
    } else {
      const line = theme_exports.theme.fg("customMessageLabel", `\x1B[1m[skill]\x1B[22m `) + theme_exports.theme.fg("customMessageText", this.skillBlock.name) + theme_exports.theme.fg("dim", ` (${keyText("app.tools.expand")} to expand)`);
      content.addChild(new Text(line, 0, 0));
    }
    this.addChild(new MouseRegion(content, (event) => {
      if (event.type !== "click" || event.button !== "left")
        return void 0;
      this.setExpanded(!this.expanded);
      return { handled: true };
    }));
  }
};

// target/vendor-pi/dist/modes/interactive/components/theme-selector.js
var THEME_SELECT_LIST_LAYOUT = {
  minPrimaryColumnWidth: 12,
  maxPrimaryColumnWidth: 32
};
var ThemeSelectorComponent = class extends Container {
  selectList;
  onPreview;
  constructor(currentTheme, onSelect, onCancel, onPreview) {
    super();
    this.onPreview = onPreview;
    const themes = (0, theme_exports.getAvailableThemes)();
    const themeItems2 = themes.map((name) => ({
      value: name,
      label: name,
      description: name === currentTheme ? "(current)" : void 0
    }));
    this.addChild(new DynamicBorder());
    this.selectList = new SelectList(themeItems2, 10, (0, theme_exports.getSelectListTheme)(), THEME_SELECT_LIST_LAYOUT);
    const currentIndex = themes.indexOf(currentTheme);
    if (currentIndex !== -1) {
      this.selectList.setSelectedIndex(currentIndex);
    }
    this.selectList.onSelect = (item) => {
      onSelect(item.value);
    };
    this.selectList.onCancel = () => {
      onCancel();
    };
    this.selectList.onSelectionChange = (item) => {
      this.onPreview(item.value);
    };
    this.addChild(this.selectList);
    this.addChild(new DynamicBorder());
  }
  getSelectList() {
    return this.selectList;
  }
};

// target/vendor-pi/dist/modes/interactive/components/thinking-selector.js
var THINKING_SELECT_LIST_LAYOUT = {
  minPrimaryColumnWidth: 12,
  maxPrimaryColumnWidth: 32
};
var LEVEL_DESCRIPTIONS = {
  off: "No reasoning",
  minimal: "Very brief reasoning (~1k tokens)",
  low: "Light reasoning (~2k tokens)",
  medium: "Moderate reasoning (~8k tokens)",
  high: "Deep reasoning (~16k tokens)",
  xhigh: "Extra-high reasoning (~32k tokens)",
  max: "Maximum reasoning"
};
var ThinkingSelectorComponent = class extends Container {
  searchInput;
  selectList;
  selectListChildIndex;
  allItems;
  onSelect;
  onCancel;
  onSelectAsDefault;
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    this.searchInput.focused = value;
  }
  constructor(currentLevel, availableLevels, onSelect, onCancel, onSelectAsDefault, defaultThinkingLevel) {
    super();
    this.onSelect = onSelect;
    this.onCancel = onCancel;
    this.onSelectAsDefault = onSelectAsDefault;
    this.allItems = availableLevels.map((level) => ({
      value: level,
      label: `${level === currentLevel ? "\u2713 " : "  "}${level}`,
      description: level === defaultThinkingLevel ? `${LEVEL_DESCRIPTIONS[level]} \xB7 default` : LEVEL_DESCRIPTIONS[level]
    }));
    this.addChild(new DynamicBorder());
    this.addChild(new Spacer(1));
    this.addChild(new Text("Thinking Level", 0, 0));
    this.addChild(new Spacer(1));
    this.addChild(new Text(`${keyDisplayText("app.thinking.cycle")} cycles thinking levels in-session`, 0, 0));
    this.addChild(new Spacer(1));
    this.searchInput = new Input();
    this.searchInput.onSubmit = () => this.selectList.handleInput("\r");
    this.addChild(this.searchInput);
    this.addChild(new Spacer(1));
    this.selectList = this.buildSelectList(this.allItems, currentLevel);
    this.selectListChildIndex = this.children.length;
    this.addChild(this.selectList);
    this.addChild(new Spacer(1));
    this.addChild(new Text(theme_exports.theme.fg("dim", `  ${keyDisplayText("tui.select.confirm")} to select \xB7 ${keyDisplayText("app.thinking.save")} to set as default \xB7 ${keyDisplayText("tui.select.cancel")} to cancel`), 0, 0));
    this.addChild(new DynamicBorder());
  }
  buildSelectList(items, preselect) {
    const list = new SelectList(items, Math.max(1, items.length), (0, theme_exports.getSelectListTheme)(), THINKING_SELECT_LIST_LAYOUT);
    const currentIndex = items.findIndex((item) => item.value === preselect);
    if (currentIndex !== -1) {
      list.setSelectedIndex(currentIndex);
    }
    list.onSelect = (item) => this.onSelect(item.value);
    list.onCancel = () => this.onCancel();
    return list;
  }
  applyFilter(query) {
    const filtered = query ? fuzzyFilter(this.allItems, query, (item) => `${item.value} ${item.description ?? ""}`) : this.allItems;
    const selectedValue = this.selectList.getSelectedItem()?.value;
    const newList = this.buildSelectList(filtered, selectedValue);
    this.children[this.selectListChildIndex] = newList;
    this.selectList = newList;
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (kb.matches(keyData, "app.thinking.save") && this.onSelectAsDefault) {
      const item = this.selectList.getSelectedItem();
      if (item)
        this.onSelectAsDefault(item.value);
      return;
    }
    const isNav = kb.matches(keyData, "tui.select.up") || kb.matches(keyData, "tui.select.down") || kb.matches(keyData, "tui.select.confirm") || kb.matches(keyData, "tui.select.cancel");
    if (isNav) {
      this.selectList.handleInput(keyData);
      return;
    }
    this.searchInput.handleInput(keyData);
    this.applyFilter(this.searchInput.getValue());
  }
  getSelectList() {
    return this.selectList;
  }
};

// target/vendor-pi/dist/modes/interactive/components/tool-execution.js
var FALLBACK_PREVIEW_LINES = 10;
var ToolExecutionComponent = class extends Container {
  contentBox;
  contentText;
  contentTextRegion;
  selfRenderContainer;
  selfRenderHeight = 0;
  callRendererComponent;
  resultRendererComponent;
  rendererState = {};
  imageComponents = [];
  imageSpacers = [];
  toolName;
  toolCallId;
  args;
  expanded = false;
  showImages;
  imageWidthCells;
  isPartial = true;
  toolDefinition;
  ui;
  cwd;
  executionStarted = false;
  argsComplete = false;
  result;
  convertedImages = /* @__PURE__ */ new Map();
  hideComponent = false;
  constructor(toolName, toolCallId, args, options = {}, toolDefinition, ui, cwd) {
    super();
    this.toolName = toolName;
    this.toolCallId = toolCallId;
    this.args = args;
    this.toolDefinition = toolDefinition;
    this.showImages = options.showImages ?? true;
    this.imageWidthCells = options.imageWidthCells ?? 60;
    this.ui = ui;
    this.cwd = cwd;
    this.addChild(new Spacer(1));
    this.contentBox = new Box(1, 1, (text) => theme_exports.theme.bg("toolPendingBg", text));
    this.contentText = new Text("", 1, 1, (text) => theme_exports.theme.bg("toolPendingBg", text));
    this.contentTextRegion = this.createResultRegion(this.contentText);
    this.selfRenderContainer = new Container();
    if (this.hasRendererDefinition()) {
      this.addChild(this.getRenderShell() === "self" ? this.selfRenderContainer : this.contentBox);
    } else {
      this.addChild(this.contentTextRegion);
    }
    this.updateDisplay();
  }
  getCallRenderer() {
    return this.toolDefinition?.renderCall;
  }
  getResultRenderer() {
    return this.toolDefinition?.renderResult;
  }
  hasRendererDefinition() {
    return this.toolDefinition !== void 0;
  }
  getRenderShell() {
    return this.toolDefinition?.renderShell ?? "default";
  }
  getRenderContext(lastComponent) {
    return {
      args: this.args,
      toolCallId: this.toolCallId,
      invalidate: () => {
        this.invalidate();
        this.ui.requestRender();
      },
      lastComponent,
      state: this.rendererState,
      cwd: this.cwd,
      executionStarted: this.executionStarted,
      argsComplete: this.argsComplete,
      isPartial: this.isPartial,
      expanded: this.expanded,
      showImages: this.showImages,
      isError: this.result?.isError ?? false
    };
  }
  createCallFallback() {
    return new Text(formatToolCallWithArgs(this.toolName, this.args, theme_exports.theme, this.expanded), 0, 0);
  }
  createResultFallback() {
    const output = this.getTextOutput();
    if (!output) {
      return void 0;
    }
    const lines = output.split("\n");
    const displayLines = this.expanded ? lines : lines.slice(0, FALLBACK_PREVIEW_LINES);
    const remaining = lines.length - displayLines.length;
    let text = displayLines.map((line) => theme_exports.theme.fg("toolOutput", line)).join("\n");
    if (remaining > 0) {
      text += `${theme_exports.theme.fg("muted", `
... (${remaining} more lines,`)} ${keyHint("app.tools.expand", "to expand")}${theme_exports.theme.fg("muted", ")")}`;
    }
    return new Text(text, 0, 0);
  }
  createResultRegion(component) {
    return new MouseRegion(component, (event) => {
      if (!this.result || event.type !== "click" || event.button !== "left")
        return void 0;
      this.setExpanded(!this.expanded);
      return { handled: true };
    });
  }
  updateArgs(args) {
    this.args = args;
    this.updateDisplay();
  }
  markExecutionStarted() {
    this.executionStarted = true;
    this.updateDisplay();
    this.ui.requestRender();
  }
  setArgsComplete() {
    this.argsComplete = true;
    this.updateDisplay();
    this.ui.requestRender();
  }
  updateResult(result, isPartial = false) {
    this.result = result;
    this.isPartial = isPartial;
    this.updateDisplay();
    this.maybeConvertImagesForKitty();
  }
  maybeConvertImagesForKitty() {
    const caps = getCapabilities();
    if (caps.images !== "kitty")
      return;
    if (!this.result)
      return;
    const imageBlocks = this.result.content.filter((c) => c.type === "image");
    for (let i = 0; i < imageBlocks.length; i++) {
      const img = imageBlocks[i];
      if (!img.data || !img.mimeType)
        continue;
      const sourceData = img.data;
      const sourceMimeType = img.mimeType;
      if (sourceMimeType === "image/png")
        continue;
      const cached = this.convertedImages.get(i);
      if (cached?.sourceData === sourceData && cached.sourceMimeType === sourceMimeType)
        continue;
      const index = i;
      (0, image_convert_exports.convertToPng)(sourceData, sourceMimeType).then((converted) => {
        const currentImage = this.result?.content.filter((content) => content.type === "image")[index];
        if (!converted || currentImage?.data !== sourceData || currentImage.mimeType !== sourceMimeType)
          return;
        this.convertedImages.set(index, {
          sourceData,
          sourceMimeType,
          ...converted
        });
        this.updateDisplay();
        this.ui.requestRender();
      });
    }
  }
  setExpanded(expanded) {
    this.expanded = expanded;
    this.updateDisplay();
  }
  setShowImages(show) {
    this.showImages = show;
    this.updateDisplay();
  }
  setImageWidthCells(width) {
    this.imageWidthCells = Math.max(1, Math.floor(width));
    this.updateDisplay();
  }
  invalidate() {
    super.invalidate();
    this.updateDisplay();
  }
  render(width) {
    if (this.hideComponent) {
      return [];
    }
    if (this.hasRendererDefinition() && this.getRenderShell() === "self") {
      const contentLines = this.selfRenderContainer.render(width);
      this.selfRenderHeight = contentLines.length;
      if (contentLines.length === 0 && this.imageComponents.length === 0) {
        return [];
      }
      const lines = [];
      if (contentLines.length > 0) {
        lines.push("");
        lines.push(...contentLines);
      }
      for (let i = 0; i < this.imageComponents.length; i++) {
        const spacer = this.imageSpacers[i];
        if (spacer) {
          lines.push(...spacer.render(width));
        }
        const imageComponent = this.imageComponents[i];
        if (imageComponent) {
          lines.push(...imageComponent.render(width));
        }
      }
      return lines;
    }
    return super.render(width);
  }
  handleMouse(event) {
    if (!this.hasRendererDefinition() || this.getRenderShell() !== "self")
      return super.handleMouse(event);
    if (event.y <= 0 || event.y > this.selfRenderHeight)
      return void 0;
    return this.selfRenderContainer.handleMouse({
      ...event,
      y: event.y - 1,
      height: this.selfRenderHeight
    });
  }
  updateDisplay() {
    const bgFn = this.isPartial ? (text) => theme_exports.theme.bg("toolPendingBg", text) : this.result?.isError ? (text) => theme_exports.theme.bg("toolErrorBg", text) : (text) => theme_exports.theme.bg("toolSuccessBg", text);
    let hasContent = false;
    this.hideComponent = false;
    if (this.hasRendererDefinition()) {
      const renderContainer = this.getRenderShell() === "self" ? this.selfRenderContainer : this.contentBox;
      if (renderContainer instanceof Box) {
        renderContainer.setBgFn(bgFn);
      }
      renderContainer.clear();
      const callRenderer = this.getCallRenderer();
      if (!callRenderer) {
        renderContainer.addChild(this.createResultRegion(this.createCallFallback()));
        hasContent = true;
      } else {
        try {
          const component = callRenderer(this.args, theme_exports.theme, this.getRenderContext(this.callRendererComponent));
          this.callRendererComponent = component;
          renderContainer.addChild(this.createResultRegion(component));
          hasContent = true;
        } catch {
          this.callRendererComponent = void 0;
          renderContainer.addChild(this.createResultRegion(this.createCallFallback()));
          hasContent = true;
        }
      }
      if (this.result) {
        const resultRenderer = this.getResultRenderer();
        if (!resultRenderer) {
          const component = this.createResultFallback();
          if (component) {
            renderContainer.addChild(this.createResultRegion(component));
            hasContent = true;
          }
        } else {
          try {
            const component = resultRenderer({ content: this.result.content, details: this.result.details }, { expanded: this.expanded, isPartial: this.isPartial }, theme_exports.theme, this.getRenderContext(this.resultRendererComponent));
            this.resultRendererComponent = component;
            renderContainer.addChild(this.createResultRegion(component));
            hasContent = true;
          } catch {
            this.resultRendererComponent = void 0;
            const component = this.createResultFallback();
            if (component) {
              renderContainer.addChild(this.createResultRegion(component));
              hasContent = true;
            }
          }
        }
      }
    } else {
      this.contentText.setCustomBgFn(bgFn);
      this.contentText.setText(this.formatToolExecution());
      hasContent = true;
    }
    for (const img of this.imageComponents) {
      this.removeChild(img);
    }
    this.imageComponents = [];
    for (const spacer of this.imageSpacers) {
      this.removeChild(spacer);
    }
    this.imageSpacers = [];
    if (this.result) {
      const imageBlocks = this.result.content.filter((c) => c.type === "image");
      const caps = getCapabilities();
      for (let i = 0; i < imageBlocks.length; i++) {
        const img = imageBlocks[i];
        if (caps.images && this.showImages && img.data && img.mimeType) {
          const cached = this.convertedImages.get(i);
          const converted = cached?.sourceData === img.data && cached.sourceMimeType === img.mimeType ? cached : void 0;
          const imageData = converted?.data ?? img.data;
          const imageMimeType = converted?.mimeType ?? img.mimeType;
          if (caps.images === "kitty" && imageMimeType !== "image/png")
            continue;
          const spacer = new Spacer(1);
          this.addChild(spacer);
          this.imageSpacers.push(spacer);
          const imageComponent = new Image(imageData, imageMimeType, { fallbackColor: (s) => theme_exports.theme.fg("toolOutput", s) }, { maxWidthCells: this.imageWidthCells });
          this.imageComponents.push(imageComponent);
          this.addChild(imageComponent);
        }
      }
    }
    if (this.hasRendererDefinition() && !hasContent && this.imageComponents.length === 0) {
      this.hideComponent = true;
    }
  }
  getTextOutput() {
    return getTextOutput(this.result, this.showImages);
  }
  formatToolExecution() {
    let text = theme_exports.theme.fg("toolTitle", theme_exports.theme.bold(this.toolName));
    const content = JSON.stringify(this.args, null, 2);
    if (content) {
      text += `

${content}`;
    }
    const output = this.getTextOutput();
    if (output) {
      text += `
${output}`;
    }
    return text;
  }
};

// target/vendor-pi/dist/modes/interactive/components/tree-selector.js
var TREE_GUTTER_WIDTH = 2;
var MIN_VISIBLE_ANCHOR_CONTENT_WIDTH = 4;
var MAX_VISIBLE_ANCHOR_CONTENT_WIDTH = 20;
var MIN_ANCHOR_CONTEXT_WIDTH = 2;
var MAX_ANCHOR_CONTEXT_WIDTH = 12;
function renderHorizontalViewport(rows, width) {
  const viewportWidth = Math.max(0, width - TREE_GUTTER_WIDTH);
  const maxBodyWidth = rows.reduce((max, row) => Math.max(max, row.bodyWidth), 0);
  const maxHorizontalScroll = Math.max(0, maxBodyWidth - viewportWidth);
  const selectedRow = rows.find((row) => row.isSelected);
  let horizontalScroll = 0;
  if (selectedRow && maxHorizontalScroll > 0) {
    const minVisibleAnchorContentWidth = Math.min(MAX_VISIBLE_ANCHOR_CONTENT_WIDTH, Math.max(MIN_VISIBLE_ANCHOR_CONTENT_WIDTH, Math.floor(viewportWidth / 3)));
    if (selectedRow.anchorCol > viewportWidth - minVisibleAnchorContentWidth) {
      const anchorContextWidth = Math.min(MAX_ANCHOR_CONTEXT_WIDTH, Math.max(MIN_ANCHOR_CONTEXT_WIDTH, Math.floor(viewportWidth / 4)));
      horizontalScroll = Math.min(maxHorizontalScroll, selectedRow.anchorCol - anchorContextWidth);
    }
  }
  return rows.map((row) => {
    const line = horizontalScroll > 0 ? `${row.gutter}${sliceByColumn(row.body, horizontalScroll, viewportWidth, true)}\x1B[0m` : row.gutter + row.body;
    return truncateToWidth(line, width, "");
  });
}
var TreeList = class {
  flatNodes = [];
  filteredNodes = [];
  selectedIndex = 0;
  currentLeafId;
  maxVisibleLines;
  filterMode = "default";
  searchQuery = "";
  toolCallMap = /* @__PURE__ */ new Map();
  multipleRoots = false;
  showLabelTimestamps = false;
  activePathIds = /* @__PURE__ */ new Set();
  visibleParentMap = /* @__PURE__ */ new Map();
  visibleChildrenMap = /* @__PURE__ */ new Map();
  lastSelectedId = null;
  foldedNodes = /* @__PURE__ */ new Set();
  onSelect;
  onCancel;
  onCopy;
  onLabelEdit;
  constructor(tree, currentLeafId, maxVisibleLines, initialSelectedId, initialFilterMode) {
    this.currentLeafId = currentLeafId;
    this.maxVisibleLines = maxVisibleLines;
    this.filterMode = initialFilterMode ?? "default";
    this.multipleRoots = tree.length > 1;
    this.flatNodes = this.flattenTree(tree);
    this.buildActivePath();
    this.applyFilter();
    const targetId = initialSelectedId ?? currentLeafId;
    this.selectedIndex = this.findNearestVisibleIndex(targetId);
    this.lastSelectedId = this.filteredNodes[this.selectedIndex]?.node.entry.id ?? null;
  }
  /**
   * Find the index of the nearest visible entry, walking up the parent chain if needed.
   * Returns the index in filteredNodes, or the last index as fallback.
   */
  findNearestVisibleIndex(entryId) {
    if (this.filteredNodes.length === 0)
      return 0;
    const entryMap = /* @__PURE__ */ new Map();
    for (const flatNode of this.flatNodes) {
      entryMap.set(flatNode.node.entry.id, flatNode);
    }
    const visibleIdToIndex = new Map(this.filteredNodes.map((node, i) => [node.node.entry.id, i]));
    let currentId = entryId;
    while (currentId !== null) {
      const index = visibleIdToIndex.get(currentId);
      if (index !== void 0)
        return index;
      const node = entryMap.get(currentId);
      if (!node)
        break;
      currentId = node.node.entry.parentId ?? null;
    }
    return this.filteredNodes.length - 1;
  }
  /** Build the set of entry IDs on the path from root to current leaf */
  buildActivePath() {
    this.activePathIds.clear();
    if (!this.currentLeafId)
      return;
    const entryMap = /* @__PURE__ */ new Map();
    for (const flatNode of this.flatNodes) {
      entryMap.set(flatNode.node.entry.id, flatNode);
    }
    let currentId = this.currentLeafId;
    while (currentId) {
      this.activePathIds.add(currentId);
      const node = entryMap.get(currentId);
      if (!node)
        break;
      currentId = node.node.entry.parentId ?? null;
    }
  }
  flattenTree(roots) {
    const result = [];
    this.toolCallMap.clear();
    const stack = [];
    const containsActive = /* @__PURE__ */ new Map();
    const leafId = this.currentLeafId;
    {
      const allNodes = [];
      const preOrderStack = [...roots];
      while (preOrderStack.length > 0) {
        const node = preOrderStack.pop();
        allNodes.push(node);
        for (let i = node.children.length - 1; i >= 0; i--) {
          preOrderStack.push(node.children[i]);
        }
      }
      for (let i = allNodes.length - 1; i >= 0; i--) {
        const node = allNodes[i];
        let has = leafId !== null && node.entry.id === leafId;
        for (const child of node.children) {
          if (containsActive.get(child)) {
            has = true;
          }
        }
        containsActive.set(node, has);
      }
    }
    const multipleRoots = roots.length > 1;
    const orderedRoots = [...roots].sort((a, b) => Number(containsActive.get(b)) - Number(containsActive.get(a)));
    for (let i = orderedRoots.length - 1; i >= 0; i--) {
      const isLast = i === orderedRoots.length - 1;
      stack.push([orderedRoots[i], multipleRoots ? 1 : 0, multipleRoots, multipleRoots, isLast, [], multipleRoots]);
    }
    while (stack.length > 0) {
      const [node, indent, justBranched, showConnector, isLast, gutters, isVirtualRootChild] = stack.pop();
      const entry = node.entry;
      if (entry.type === "message" && entry.message.role === "assistant") {
        const content = entry.message.content;
        if (Array.isArray(content)) {
          for (const block of content) {
            if (typeof block === "object" && block !== null && "type" in block && block.type === "toolCall") {
              const tc = block;
              this.toolCallMap.set(tc.id, { name: tc.name, arguments: tc.arguments });
            }
          }
        }
      }
      result.push({ node, indent, showConnector, isLast, gutters, isVirtualRootChild });
      const children = node.children;
      const multipleChildren = children.length > 1;
      const orderedChildren = (() => {
        const prioritized = [];
        const rest = [];
        for (const child of children) {
          if (containsActive.get(child)) {
            prioritized.push(child);
          } else {
            rest.push(child);
          }
        }
        return [...prioritized, ...rest];
      })();
      let childIndent;
      if (multipleChildren) {
        childIndent = indent + 1;
      } else if (justBranched && indent > 0) {
        childIndent = indent + 1;
      } else {
        childIndent = indent;
      }
      const connectorDisplayed = showConnector && !isVirtualRootChild;
      const currentDisplayIndent = this.multipleRoots ? Math.max(0, indent - 1) : indent;
      const connectorPosition = Math.max(0, currentDisplayIndent - 1);
      const childGutters = connectorDisplayed ? [...gutters, { position: connectorPosition, show: !isLast }] : gutters;
      for (let i = orderedChildren.length - 1; i >= 0; i--) {
        const childIsLast = i === orderedChildren.length - 1;
        stack.push([
          orderedChildren[i],
          childIndent,
          multipleChildren,
          multipleChildren,
          childIsLast,
          childGutters,
          false
        ]);
      }
    }
    return result;
  }
  applyFilter() {
    if (this.filteredNodes.length > 0) {
      this.lastSelectedId = this.filteredNodes[this.selectedIndex]?.node.entry.id ?? this.lastSelectedId;
    }
    const searchTokens = this.searchQuery.toLowerCase().split(/\s+/).filter(Boolean);
    this.filteredNodes = this.flatNodes.filter((flatNode) => {
      const entry = flatNode.node.entry;
      if (entry.type === "usage")
        return false;
      const isCurrentLeaf = entry.id === this.currentLeafId;
      if (entry.type === "message" && entry.message.role === "assistant" && !isCurrentLeaf) {
        const msg = entry.message;
        const hasText = this.hasTextContent(msg.content);
        const isErrorOrAborted = msg.stopReason && msg.stopReason !== "stop" && msg.stopReason !== "toolUse";
        if (!hasText && !isErrorOrAborted) {
          return false;
        }
      }
      let passesFilter = true;
      const isSettingsEntry = entry.type === "label" || entry.type === "context_edit" || entry.type === "custom" || entry.type === "model_change" || entry.type === "thinking_level_change" || entry.type === "session_info";
      switch (this.filterMode) {
        case "user-only":
          passesFilter = entry.type === "message" && entry.message.role === "user";
          break;
        case "no-tools":
          passesFilter = !isSettingsEntry && !(entry.type === "message" && entry.message.role === "toolResult");
          break;
        case "labeled-only":
          passesFilter = flatNode.node.label !== void 0;
          break;
        case "all":
          passesFilter = true;
          break;
        default:
          passesFilter = !isSettingsEntry;
          break;
      }
      if (!passesFilter)
        return false;
      if (searchTokens.length > 0) {
        const nodeText = this.getSearchableText(flatNode.node).toLowerCase();
        return searchTokens.every((token) => nodeText.includes(token));
      }
      return true;
    });
    if (this.foldedNodes.size > 0) {
      const skipSet = /* @__PURE__ */ new Set();
      for (const flatNode of this.flatNodes) {
        const { id, parentId } = flatNode.node.entry;
        if (parentId != null && (this.foldedNodes.has(parentId) || skipSet.has(parentId))) {
          skipSet.add(id);
        }
      }
      this.filteredNodes = this.filteredNodes.filter((flatNode) => !skipSet.has(flatNode.node.entry.id));
    }
    this.recalculateVisualStructure();
    if (this.lastSelectedId) {
      this.selectedIndex = this.findNearestVisibleIndex(this.lastSelectedId);
    } else if (this.selectedIndex >= this.filteredNodes.length) {
      this.selectedIndex = Math.max(0, this.filteredNodes.length - 1);
    }
    if (this.filteredNodes.length > 0) {
      this.lastSelectedId = this.filteredNodes[this.selectedIndex]?.node.entry.id ?? this.lastSelectedId;
    }
  }
  /**
   * Recompute indentation/connectors for the filtered view
   *
   * Filtering can hide intermediate entries; descendants attach to the nearest visible ancestor.
   * Keep indentation semantics aligned with flattenTree() so single-child chains don't drift right.
   */
  recalculateVisualStructure() {
    if (this.filteredNodes.length === 0)
      return;
    const visibleIds = new Set(this.filteredNodes.map((n) => n.node.entry.id));
    const entryMap = /* @__PURE__ */ new Map();
    for (const flatNode of this.flatNodes) {
      entryMap.set(flatNode.node.entry.id, flatNode);
    }
    const findVisibleAncestor = (nodeId) => {
      let currentId = entryMap.get(nodeId)?.node.entry.parentId ?? null;
      while (currentId !== null) {
        if (visibleIds.has(currentId)) {
          return currentId;
        }
        currentId = entryMap.get(currentId)?.node.entry.parentId ?? null;
      }
      return null;
    };
    const visibleParent = /* @__PURE__ */ new Map();
    const visibleChildren = /* @__PURE__ */ new Map();
    visibleChildren.set(null, []);
    for (const flatNode of this.filteredNodes) {
      const nodeId = flatNode.node.entry.id;
      const ancestorId = findVisibleAncestor(nodeId);
      visibleParent.set(nodeId, ancestorId);
      if (!visibleChildren.has(ancestorId)) {
        visibleChildren.set(ancestorId, []);
      }
      visibleChildren.get(ancestorId).push(nodeId);
    }
    const visibleRootIds = visibleChildren.get(null);
    this.multipleRoots = visibleRootIds.length > 1;
    const filteredNodeMap = /* @__PURE__ */ new Map();
    for (const flatNode of this.filteredNodes) {
      filteredNodeMap.set(flatNode.node.entry.id, flatNode);
    }
    const stack = [];
    for (let i = visibleRootIds.length - 1; i >= 0; i--) {
      const isLast = i === visibleRootIds.length - 1;
      stack.push([
        visibleRootIds[i],
        this.multipleRoots ? 1 : 0,
        this.multipleRoots,
        this.multipleRoots,
        isLast,
        [],
        this.multipleRoots
      ]);
    }
    while (stack.length > 0) {
      const [nodeId, indent, justBranched, showConnector, isLast, gutters, isVirtualRootChild] = stack.pop();
      const flatNode = filteredNodeMap.get(nodeId);
      if (!flatNode)
        continue;
      flatNode.indent = indent;
      flatNode.showConnector = showConnector;
      flatNode.isLast = isLast;
      flatNode.gutters = gutters;
      flatNode.isVirtualRootChild = isVirtualRootChild;
      const children = visibleChildren.get(nodeId) || [];
      const multipleChildren = children.length > 1;
      let childIndent;
      if (multipleChildren) {
        childIndent = indent + 1;
      } else if (justBranched && indent > 0) {
        childIndent = indent + 1;
      } else {
        childIndent = indent;
      }
      const connectorDisplayed = showConnector && !isVirtualRootChild;
      const currentDisplayIndent = this.multipleRoots ? Math.max(0, indent - 1) : indent;
      const connectorPosition = Math.max(0, currentDisplayIndent - 1);
      const childGutters = connectorDisplayed ? [...gutters, { position: connectorPosition, show: !isLast }] : gutters;
      for (let i = children.length - 1; i >= 0; i--) {
        const childIsLast = i === children.length - 1;
        stack.push([
          children[i],
          childIndent,
          multipleChildren,
          multipleChildren,
          childIsLast,
          childGutters,
          false
        ]);
      }
    }
    this.visibleParentMap = visibleParent;
    this.visibleChildrenMap = visibleChildren;
  }
  /** Get searchable text content from a node */
  getSearchableText(node) {
    const entry = node.entry;
    const parts = [];
    if (node.label) {
      parts.push(node.label);
    }
    switch (entry.type) {
      case "message": {
        const msg = entry.message;
        parts.push(msg.role);
        if ("content" in msg && msg.content) {
          parts.push(this.extractContent(msg.content));
        }
        if (msg.role === "bashExecution") {
          const bashMsg = msg;
          if (bashMsg.command)
            parts.push(bashMsg.command);
        }
        break;
      }
      case "custom_message": {
        parts.push(entry.customType);
        if (typeof entry.content === "string") {
          parts.push(entry.content);
        } else {
          parts.push(this.extractContent(entry.content));
        }
        break;
      }
      case "compaction":
        parts.push("compaction");
        break;
      case "branch_summary":
        parts.push("branch summary", entry.summary);
        break;
      case "session_info":
        parts.push("title");
        if (entry.name)
          parts.push(entry.name);
        break;
      case "model_change":
        parts.push("model", entry.modelId);
        break;
      case "thinking_level_change":
        parts.push("thinking", entry.thinkingLevel);
        break;
      case "custom":
        parts.push("custom", entry.customType);
        break;
      case "context_edit":
        parts.push("context edit", entry.replacement === null ? "omit" : "replace", entry.targetId);
        break;
      case "label":
        parts.push("label", entry.label ?? "");
        break;
    }
    return parts.join(" ");
  }
  invalidate() {
  }
  getSearchQuery() {
    return this.searchQuery;
  }
  getSelectedNode() {
    return this.filteredNodes[this.selectedIndex]?.node;
  }
  copySelected() {
    const node = this.getSelectedNode();
    this.onCopy?.(node ? this.getEntryCopyText(node) : void 0);
  }
  updateNodeLabel(entryId, label, labelTimestamp) {
    for (const flatNode of this.flatNodes) {
      if (flatNode.node.entry.id === entryId) {
        flatNode.node.label = label;
        flatNode.node.labelTimestamp = label ? labelTimestamp ?? (/* @__PURE__ */ new Date()).toISOString() : void 0;
        break;
      }
    }
  }
  getStatusLabels() {
    let labels = "";
    switch (this.filterMode) {
      case "no-tools":
        labels += " [no-tools]";
        break;
      case "user-only":
        labels += " [user]";
        break;
      case "labeled-only":
        labels += " [labeled]";
        break;
      case "all":
        labels += " [all]";
        break;
    }
    if (this.showLabelTimestamps) {
      labels += " [+label time]";
    }
    return labels;
  }
  render(width) {
    const lines = [];
    if (this.filteredNodes.length === 0) {
      lines.push(truncateToWidth(theme_exports.theme.fg("muted", "  No entries found"), width));
      lines.push(truncateToWidth(theme_exports.theme.fg("muted", `  (0/0)${this.getStatusLabels()}`), width));
      return lines;
    }
    const startIndex = Math.max(0, Math.min(this.selectedIndex - Math.floor(this.maxVisibleLines / 2), this.filteredNodes.length - this.maxVisibleLines));
    const endIndex = Math.min(startIndex + this.maxVisibleLines, this.filteredNodes.length);
    const renderedRows = [];
    for (let i = startIndex; i < endIndex; i++) {
      const flatNode = this.filteredNodes[i];
      const entry = flatNode.node.entry;
      const isSelected = i === this.selectedIndex;
      const cursor = isSelected ? theme_exports.theme.fg("accent", "\u203A ") : "  ";
      const displayIndent = this.multipleRoots ? Math.max(0, flatNode.indent - 1) : flatNode.indent;
      const connector = flatNode.showConnector && !flatNode.isVirtualRootChild ? flatNode.isLast ? "\u2514\u2500 " : "\u251C\u2500 " : "";
      const connectorPosition = connector ? displayIndent - 1 : -1;
      const totalChars = displayIndent * 3;
      const prefixChars = [];
      const isFolded = this.foldedNodes.has(entry.id);
      for (let i2 = 0; i2 < totalChars; i2++) {
        const level = Math.floor(i2 / 3);
        const posInLevel = i2 % 3;
        const gutter2 = flatNode.gutters.find((g) => g.position === level);
        if (gutter2) {
          if (posInLevel === 0) {
            prefixChars.push(gutter2.show ? "\u2502" : " ");
          } else {
            prefixChars.push(" ");
          }
        } else if (connector && level === connectorPosition) {
          if (posInLevel === 0) {
            prefixChars.push(flatNode.isLast ? "\u2514" : "\u251C");
          } else if (posInLevel === 1) {
            const foldable = this.isFoldable(entry.id);
            prefixChars.push(isFolded ? "\u229E" : foldable ? "\u229F" : "\u2500");
          } else {
            prefixChars.push(" ");
          }
        } else {
          prefixChars.push(" ");
        }
      }
      const prefix = prefixChars.join("");
      const showsFoldInConnector = flatNode.showConnector && !flatNode.isVirtualRootChild;
      const foldMarker = isFolded && !showsFoldInConnector ? theme_exports.theme.fg("accent", "\u229E ") : "";
      const isOnActivePath = this.activePathIds.has(entry.id);
      const pathMarker = isOnActivePath ? theme_exports.theme.fg("accent", "\u2022 ") : "";
      const label = flatNode.node.label ? theme_exports.theme.fg("warning", `[${flatNode.node.label}] `) : "";
      const labelTimestamp = this.showLabelTimestamps && flatNode.node.label && flatNode.node.labelTimestamp ? theme_exports.theme.fg("muted", `${this.formatLabelTimestamp(flatNode.node.labelTimestamp)} `) : "";
      const content = this.getEntryDisplayText(flatNode.node, isSelected);
      const prefixPart = theme_exports.theme.fg("dim", prefix) + foldMarker + pathMarker;
      const anchorCol = visibleWidth(prefixPart);
      let gutter = cursor;
      let body = prefixPart + label + labelTimestamp + content;
      if (isSelected) {
        gutter = theme_exports.theme.bg("selectedBg", gutter);
        body = theme_exports.theme.bg("selectedBg", body);
      }
      renderedRows.push({ gutter, body, anchorCol, bodyWidth: visibleWidth(body), isSelected });
    }
    lines.push(...renderHorizontalViewport(renderedRows, width));
    lines.push(truncateToWidth(theme_exports.theme.fg("muted", `  (${this.selectedIndex + 1}/${this.filteredNodes.length})${this.getStatusLabels()}`), width));
    return lines;
  }
  getEntryDisplayText(node, isSelected) {
    const entry = node.entry;
    let result;
    const normalize = (s) => s.replace(/[\n\t]/g, " ").trim();
    switch (entry.type) {
      case "message": {
        const msg = entry.message;
        const role = msg.role;
        if (role === "user") {
          const msgWithContent = msg;
          const content = normalize(this.extractContent(msgWithContent.content));
          result = theme_exports.theme.fg("accent", "user: ") + content;
        } else if (role === "assistant") {
          const msgWithContent = msg;
          const textContent = normalize(this.extractContent(msgWithContent.content));
          if (textContent) {
            result = theme_exports.theme.fg("success", "assistant: ") + textContent;
          } else if (msgWithContent.stopReason === "aborted") {
            result = theme_exports.theme.fg("success", "assistant: ") + theme_exports.theme.fg("muted", "(aborted)");
          } else if (msgWithContent.errorMessage) {
            const errMsg = normalize(msgWithContent.errorMessage).slice(0, 80);
            result = theme_exports.theme.fg("success", "assistant: ") + theme_exports.theme.fg("error", errMsg);
          } else {
            result = theme_exports.theme.fg("success", "assistant: ") + theme_exports.theme.fg("muted", "(no content)");
          }
        } else if (role === "toolResult") {
          const toolMsg = msg;
          const toolCall = toolMsg.toolCallId ? this.toolCallMap.get(toolMsg.toolCallId) : void 0;
          if (toolCall) {
            result = theme_exports.theme.fg("muted", this.formatToolCall(toolCall.name, toolCall.arguments));
          } else {
            result = theme_exports.theme.fg("muted", `[${toolMsg.toolName ?? "tool"}]`);
          }
        } else if (role === "bashExecution") {
          const bashMsg = msg;
          result = theme_exports.theme.fg("dim", `[bash]: ${normalize(bashMsg.command ?? "")}`);
        } else {
          result = theme_exports.theme.fg("dim", `[${role}]`);
        }
        break;
      }
      case "custom_message": {
        const content = typeof entry.content === "string" ? entry.content : entry.content.filter((c) => c.type === "text").map((c) => c.text).join("");
        result = theme_exports.theme.fg("customMessageLabel", `[${entry.customType}]: `) + normalize(content);
        break;
      }
      case "compaction": {
        const tokens = Math.round(entry.tokensBefore / 1e3);
        result = theme_exports.theme.fg("borderAccent", `[compaction: ${tokens}k tokens]`);
        break;
      }
      case "branch_summary":
        result = theme_exports.theme.fg("warning", `[branch summary]: `) + normalize(entry.summary);
        break;
      case "model_change":
        result = theme_exports.theme.fg("dim", `[model: ${entry.modelId}]`);
        break;
      case "thinking_level_change":
        result = theme_exports.theme.fg("dim", `[thinking: ${entry.thinkingLevel}]`);
        break;
      case "custom":
        result = theme_exports.theme.fg("dim", `[custom: ${entry.customType}]`);
        break;
      case "context_edit":
        result = theme_exports.theme.fg("dim", `[context ${entry.replacement === null ? "omit" : "replace"}: ${entry.targetId}]`);
        break;
      case "label":
        result = theme_exports.theme.fg("dim", `[label: ${entry.label ?? "(cleared)"}]`);
        break;
      case "session_info":
        result = entry.name ? [theme_exports.theme.fg("dim", "[title: "), theme_exports.theme.fg("dim", entry.name), theme_exports.theme.fg("dim", "]")].join("") : [theme_exports.theme.fg("dim", "[title: "), theme_exports.theme.italic(theme_exports.theme.fg("dim", "empty")), theme_exports.theme.fg("dim", "]")].join("");
        break;
      default:
        result = "";
    }
    return isSelected ? theme_exports.theme.bold(result) : result;
  }
  formatLabelTimestamp(timestamp) {
    const date = new Date(timestamp);
    const now = /* @__PURE__ */ new Date();
    const hours = date.getHours().toString().padStart(2, "0");
    const minutes = date.getMinutes().toString().padStart(2, "0");
    const time = `${hours}:${minutes}`;
    if (date.getFullYear() === now.getFullYear() && date.getMonth() === now.getMonth() && date.getDate() === now.getDate()) {
      return time;
    }
    const month = date.getMonth() + 1;
    const day = date.getDate();
    if (date.getFullYear() === now.getFullYear()) {
      return `${month}/${day} ${time}`;
    }
    const year = date.getFullYear().toString().slice(-2);
    return `${year}/${month}/${day} ${time}`;
  }
  extractContent(content) {
    return this.extractFullContent(content).slice(0, 200);
  }
  extractFullContent(content) {
    if (typeof content === "string")
      return content;
    if (!Array.isArray(content))
      return "";
    let result = "";
    for (const block of content) {
      if (typeof block === "object" && block !== null && "type" in block && block.type === "text") {
        result += block.text;
      }
    }
    return result;
  }
  getEntryCopyText(node) {
    const entry = node.entry;
    let text;
    switch (entry.type) {
      case "message":
        if (entry.message.role === "bashExecution") {
          text = entry.message.command;
        } else if ("content" in entry.message) {
          text = this.extractFullContent(entry.message.content);
          if (!text && entry.message.role === "assistant") {
            text = entry.message.errorMessage;
          }
        }
        break;
      case "custom_message":
        text = this.extractFullContent(entry.content);
        break;
      case "compaction":
        text = entry.summary;
        break;
      case "branch_summary":
        text = entry.summary;
        break;
    }
    return text?.trim() ? text : void 0;
  }
  hasTextContent(content) {
    if (typeof content === "string")
      return content.trim().length > 0;
    if (Array.isArray(content)) {
      for (const c of content) {
        if (typeof c === "object" && c !== null && "type" in c && c.type === "text") {
          const text = c.text;
          if (text && text.trim().length > 0)
            return true;
        }
      }
    }
    return false;
  }
  formatToolCall(name, args) {
    const shortenPath3 = (p) => {
      const home = process.env.HOME || process.env.USERPROFILE || "";
      if (home && p.startsWith(home))
        return `~${p.slice(home.length)}`;
      return p;
    };
    switch (name) {
      case "read": {
        const path4 = shortenPath3(String(args.path || args.file_path || ""));
        const offset = args.offset;
        const limit = args.limit;
        let display = path4;
        if (offset !== void 0 || limit !== void 0) {
          const start = offset ?? 1;
          const end = limit !== void 0 ? start + limit - 1 : "";
          display += `:${start}${end ? `-${end}` : ""}`;
        }
        return `[read: ${display}]`;
      }
      case "write": {
        const path4 = shortenPath3(String(args.path || args.file_path || ""));
        return `[write: ${path4}]`;
      }
      case "edit": {
        const path4 = shortenPath3(String(args.path || args.file_path || ""));
        return `[edit: ${path4}]`;
      }
      case "bash": {
        const rawCmd = String(args.command || "");
        const cmd = rawCmd.replace(/[\n\t]/g, " ").trim().slice(0, 50);
        return `[bash: ${cmd}${rawCmd.length > 50 ? "..." : ""}]`;
      }
      case "grep": {
        const pattern = String(args.pattern || "");
        const path4 = shortenPath3(String(args.path || "."));
        return `[grep: /${pattern}/ in ${path4}]`;
      }
      case "find": {
        const pattern = String(args.pattern || "");
        const path4 = shortenPath3(String(args.path || "."));
        return `[find: ${pattern} in ${path4}]`;
      }
      case "ls": {
        const path4 = shortenPath3(String(args.path || "."));
        return `[ls: ${path4}]`;
      }
      default: {
        const argsStr = JSON.stringify(args).slice(0, 40);
        return `[${name}: ${argsStr}${JSON.stringify(args).length > 40 ? "..." : ""}]`;
      }
    }
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (kb.matches(keyData, "tui.select.up")) {
      this.selectedIndex = this.selectedIndex === 0 ? this.filteredNodes.length - 1 : this.selectedIndex - 1;
    } else if (kb.matches(keyData, "tui.select.down")) {
      this.selectedIndex = this.selectedIndex === this.filteredNodes.length - 1 ? 0 : this.selectedIndex + 1;
    } else if (kb.matches(keyData, "app.tree.foldOrUp")) {
      const currentId = this.filteredNodes[this.selectedIndex]?.node.entry.id;
      if (currentId && this.isFoldable(currentId) && !this.foldedNodes.has(currentId)) {
        this.foldedNodes.add(currentId);
        this.applyFilter();
      } else {
        this.selectedIndex = this.findBranchSegmentStart("up");
      }
    } else if (kb.matches(keyData, "app.tree.unfoldOrDown")) {
      const currentId = this.filteredNodes[this.selectedIndex]?.node.entry.id;
      if (currentId && this.foldedNodes.has(currentId)) {
        this.foldedNodes.delete(currentId);
        this.applyFilter();
      } else {
        this.selectedIndex = this.findBranchSegmentStart("down");
      }
    } else if (kb.matches(keyData, "tui.editor.cursorLeft") || kb.matches(keyData, "tui.select.pageUp")) {
      this.selectedIndex = Math.max(0, this.selectedIndex - this.maxVisibleLines);
    } else if (kb.matches(keyData, "tui.editor.cursorRight") || kb.matches(keyData, "tui.select.pageDown")) {
      this.selectedIndex = Math.min(this.filteredNodes.length - 1, this.selectedIndex + this.maxVisibleLines);
    } else if (kb.matches(keyData, "tui.select.confirm")) {
      const selected = this.filteredNodes[this.selectedIndex];
      if (selected && this.onSelect) {
        this.onSelect(selected.node.entry.id);
      }
    } else if (kb.matches(keyData, "app.message.copy")) {
      this.copySelected();
    } else if (kb.matches(keyData, "tui.select.cancel")) {
      if (this.searchQuery) {
        this.searchQuery = "";
        this.foldedNodes.clear();
        this.applyFilter();
      } else {
        this.onCancel?.();
      }
    } else if (kb.matches(keyData, "app.tree.filter.default")) {
      this.filterMode = "default";
      this.foldedNodes.clear();
      this.applyFilter();
    } else if (kb.matches(keyData, "app.tree.filter.noTools")) {
      this.filterMode = this.filterMode === "no-tools" ? "default" : "no-tools";
      this.foldedNodes.clear();
      this.applyFilter();
    } else if (kb.matches(keyData, "app.tree.filter.userOnly")) {
      this.filterMode = this.filterMode === "user-only" ? "default" : "user-only";
      this.foldedNodes.clear();
      this.applyFilter();
    } else if (kb.matches(keyData, "app.tree.filter.labeledOnly")) {
      this.filterMode = this.filterMode === "labeled-only" ? "default" : "labeled-only";
      this.foldedNodes.clear();
      this.applyFilter();
    } else if (kb.matches(keyData, "app.tree.filter.all")) {
      this.filterMode = this.filterMode === "all" ? "default" : "all";
      this.foldedNodes.clear();
      this.applyFilter();
    } else if (kb.matches(keyData, "app.tree.filter.cycleBackward")) {
      const modes = ["default", "no-tools", "user-only", "labeled-only", "all"];
      const currentIndex = modes.indexOf(this.filterMode);
      this.filterMode = modes[(currentIndex - 1 + modes.length) % modes.length];
      this.foldedNodes.clear();
      this.applyFilter();
    } else if (kb.matches(keyData, "app.tree.filter.cycleForward")) {
      const modes = ["default", "no-tools", "user-only", "labeled-only", "all"];
      const currentIndex = modes.indexOf(this.filterMode);
      this.filterMode = modes[(currentIndex + 1) % modes.length];
      this.foldedNodes.clear();
      this.applyFilter();
    } else if (kb.matches(keyData, "tui.editor.deleteCharBackward")) {
      if (this.searchQuery.length > 0) {
        this.searchQuery = this.searchQuery.slice(0, -1);
        this.foldedNodes.clear();
        this.applyFilter();
      }
    } else if (kb.matches(keyData, "app.tree.editLabel")) {
      const selected = this.filteredNodes[this.selectedIndex];
      if (selected && this.onLabelEdit) {
        this.onLabelEdit(selected.node.entry.id, selected.node.label);
      }
    } else if (kb.matches(keyData, "app.tree.toggleLabelTimestamp")) {
      this.showLabelTimestamps = !this.showLabelTimestamps;
    } else {
      const hasControlChars = [...keyData].some((ch) => {
        const code = ch.charCodeAt(0);
        return code < 32 || code === 127 || code >= 128 && code <= 159;
      });
      if (!hasControlChars && keyData.length > 0) {
        this.searchQuery += keyData;
        this.foldedNodes.clear();
        this.applyFilter();
      }
    }
  }
  /**
   * Whether a node can be folded. A node is foldable if it has visible children
   * and is either a root (no visible parent) or a segment start (visible parent
   * has multiple visible children).
   */
  isFoldable(entryId) {
    const children = this.visibleChildrenMap.get(entryId);
    if (!children || children.length === 0)
      return false;
    const parentId = this.visibleParentMap.get(entryId);
    if (parentId === null || parentId === void 0)
      return true;
    const siblings = this.visibleChildrenMap.get(parentId);
    return siblings !== void 0 && siblings.length > 1;
  }
  /**
   * Find the index of the next branch segment start in the given direction.
   * A segment start is the first child of a branch point.
   *
   * "up" walks the visible parent chain; "down" walks visible children
   * (always following the first child).
   */
  findBranchSegmentStart(direction) {
    const selectedId = this.filteredNodes[this.selectedIndex]?.node.entry.id;
    if (!selectedId)
      return this.selectedIndex;
    const indexByEntryId = new Map(this.filteredNodes.map((node, i) => [node.node.entry.id, i]));
    let currentId = selectedId;
    if (direction === "down") {
      while (true) {
        const children = this.visibleChildrenMap.get(currentId) ?? [];
        if (children.length === 0)
          return indexByEntryId.get(currentId);
        if (children.length > 1)
          return indexByEntryId.get(children[0]);
        currentId = children[0];
      }
    }
    while (true) {
      const parentId = this.visibleParentMap.get(currentId) ?? null;
      if (parentId === null)
        return indexByEntryId.get(currentId);
      const children = this.visibleChildrenMap.get(parentId) ?? [];
      if (children.length > 1) {
        const segmentStart = indexByEntryId.get(currentId);
        if (segmentStart < this.selectedIndex) {
          return segmentStart;
        }
      }
      currentId = parentId;
    }
  }
};
var SearchLine = class {
  treeList;
  constructor(treeList) {
    this.treeList = treeList;
  }
  invalidate() {
  }
  render(width) {
    const query = this.treeList.getSearchQuery();
    if (query) {
      return [truncateToWidth(`  ${theme_exports.theme.fg("muted", "Type to search:")} ${theme_exports.theme.fg("accent", query)}`, width)];
    }
    return [truncateToWidth(`  ${theme_exports.theme.fg("muted", "Type to search:")}`, width)];
  }
  handleInput(_keyData) {
  }
};
var TreeHelp = class {
  invalidate() {
  }
  render(width) {
    const items = TREE_HELP_ITEMS.map(({ keys, label, labelFirst }) => {
      const text = formatHelpKeys(keys);
      if (!text)
        return label;
      return labelFirst ? `${label} ${text}` : `${text} ${label}`;
    });
    const availableWidth = Math.max(1, width);
    const indent = "  ";
    const separator = " \xB7 ";
    const lines = [];
    let currentLine = "";
    for (const item of items) {
      const candidate = currentLine ? `${currentLine}${separator}${item}` : visibleWidth(`${indent}${item}`) <= availableWidth ? `${indent}${item}` : item;
      if (!currentLine || visibleWidth(candidate) <= availableWidth) {
        currentLine = candidate;
        continue;
      }
      lines.push(...wrapTextWithAnsi(currentLine.trimEnd(), availableWidth));
      currentLine = visibleWidth(`${indent}${item}`) <= availableWidth ? `${indent}${item}` : item;
    }
    if (currentLine) {
      lines.push(...wrapTextWithAnsi(currentLine.trimEnd(), availableWidth));
    }
    return lines.map((line) => theme_exports.theme.fg("muted", line));
  }
};
var TREE_HELP_ITEMS = [
  { keys: ["tui.select.up", "tui.select.down"], label: "move" },
  { keys: ["tui.editor.cursorLeft", "tui.editor.cursorRight"], label: "page" },
  { keys: ["app.tree.foldOrUp", "app.tree.unfoldOrDown"], label: "branch" },
  { keys: ["app.message.copy"], label: "copy" },
  { keys: ["app.tree.editLabel"], label: "label" },
  { keys: ["app.tree.toggleLabelTimestamp"], label: "label time" },
  {
    keys: [
      "app.tree.filter.default",
      "app.tree.filter.noTools",
      "app.tree.filter.userOnly",
      "app.tree.filter.labeledOnly",
      "app.tree.filter.all"
    ],
    label: "filters",
    labelFirst: true
  },
  { keys: ["app.tree.filter.cycleForward", "app.tree.filter.cycleBackward"], label: "cycle", labelFirst: true }
];
function formatHelpKeys(keybindings) {
  const keys = [];
  for (const keybinding of keybindings) {
    const key = getKeybindings().getKeys(keybinding)[0];
    if (key !== void 0)
      keys.push(key);
  }
  if (keys.length === 0)
    return "";
  return formatKeyText(compactRawKeys(keys)).replace(/\bpageUp\b/g, "pgup").replace(/\bpageDown\b/g, "pgdn").replace(/\bup\b/g, "\u2191").replace(/\bdown\b/g, "\u2193").replace(/\bleft\b/g, "\u2190").replace(/\bright\b/g, "\u2192");
}
function compactRawKeys(keys) {
  if (keys.length === 1)
    return keys[0];
  const parts = keys.map((key) => {
    const separatorIndex = key.lastIndexOf("+");
    return separatorIndex === -1 ? { prefix: "", suffix: key } : { prefix: key.slice(0, separatorIndex + 1), suffix: key.slice(separatorIndex + 1) };
  });
  const prefix = parts[0].prefix;
  return prefix && parts.every((part) => part.prefix === prefix) ? `${prefix}${parts.map((part) => part.suffix).join("/")}` : keys.join("/");
}
var LabelInput = class {
  input;
  entryId;
  onSubmit;
  onCancel;
  // Focusable implementation - propagate to input for IME cursor positioning
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    this.input.focused = value;
  }
  constructor(entryId, currentLabel) {
    this.entryId = entryId;
    this.input = new Input();
    if (currentLabel) {
      this.input.setValue(currentLabel);
    }
  }
  invalidate() {
  }
  render(width) {
    const lines = [];
    const indent = "  ";
    const availableWidth = width - indent.length;
    lines.push(truncateToWidth(`${indent}${theme_exports.theme.fg("muted", "Label (empty to remove):")}`, width));
    lines.push(...this.input.render(availableWidth).map((line) => truncateToWidth(`${indent}${line}`, width)));
    lines.push(truncateToWidth(`${indent}${keyHint("tui.select.confirm", "save")}  ${keyHint("tui.select.cancel", "cancel")}`, width));
    return lines;
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (kb.matches(keyData, "tui.select.confirm")) {
      const value = this.input.getValue().trim();
      this.onSubmit?.(this.entryId, value || void 0);
    } else if (kb.matches(keyData, "tui.select.cancel")) {
      this.onCancel?.();
    } else {
      this.input.handleInput(keyData);
    }
  }
};
var TreeSelectorComponent = class extends Container {
  treeList;
  labelInput = null;
  labelInputContainer;
  treeContainer;
  onLabelChangeCallback;
  onCopy;
  // Focusable implementation - propagate to labelInput when active for IME cursor positioning
  _focused = false;
  get focused() {
    return this._focused;
  }
  set focused(value) {
    this._focused = value;
    if (this.labelInput) {
      this.labelInput.focused = value;
    }
  }
  constructor(tree, currentLeafId, terminalHeight, onSelect, onCancel, onLabelChange, initialSelectedId, initialFilterMode) {
    super();
    this.onLabelChangeCallback = onLabelChange;
    const maxVisibleLines = Math.max(5, Math.floor(terminalHeight / 2));
    this.treeList = new TreeList(tree, currentLeafId, maxVisibleLines, initialSelectedId, initialFilterMode);
    this.treeList.onSelect = onSelect;
    this.treeList.onCancel = onCancel;
    this.treeList.onCopy = (text) => this.onCopy?.(text);
    this.treeList.onLabelEdit = (entryId, currentLabel) => this.showLabelInput(entryId, currentLabel);
    this.treeContainer = new Container();
    this.treeContainer.addChild(this.treeList);
    this.labelInputContainer = new Container();
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder());
    this.addChild(new Text(theme_exports.theme.bold("  Session Tree"), 1, 0));
    this.addChild(new TreeHelp());
    this.addChild(new SearchLine(this.treeList));
    this.addChild(new DynamicBorder());
    this.addChild(new Spacer(1));
    this.addChild(this.treeContainer);
    this.addChild(this.labelInputContainer);
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder());
    if (tree.length === 0) {
      setTimeout(() => onCancel(), 100);
    }
  }
  showLabelInput(entryId, currentLabel) {
    this.labelInput = new LabelInput(entryId, currentLabel);
    this.labelInput.onSubmit = (id, label) => {
      this.treeList.updateNodeLabel(id, label);
      this.onLabelChangeCallback?.(id, label);
      this.hideLabelInput();
    };
    this.labelInput.onCancel = () => this.hideLabelInput();
    this.labelInput.focused = this._focused;
    this.treeContainer.clear();
    this.labelInputContainer.clear();
    this.labelInputContainer.addChild(this.labelInput);
  }
  hideLabelInput() {
    this.labelInput = null;
    this.labelInputContainer.clear();
    this.treeContainer.clear();
    this.treeContainer.addChild(this.treeList);
  }
  handleInput(keyData) {
    if (this.labelInput) {
      this.labelInput.handleInput(keyData);
    } else {
      this.treeList.handleInput(keyData);
    }
  }
  getTreeList() {
    return this.treeList;
  }
};

// target/vendor-pi/dist/modes/interactive/components/user-message.js
var OSC133_ZONE_START2 = "\x1B]133;A\x07";
var OSC133_ZONE_END2 = "\x1B]133;B\x07";
var OSC133_ZONE_FINAL2 = "\x1B]133;C\x07";
var UserMessageComponent = class extends Container {
  text;
  markdownTheme;
  outputPad;
  markdownTransformers;
  constructor(text, markdownTheme = (0, theme_exports.getMarkdownTheme)(), outputPad = 1, markdownTransformers = []) {
    super();
    this.text = text;
    this.markdownTheme = markdownTheme;
    this.outputPad = outputPad;
    this.markdownTransformers = markdownTransformers;
    this.rebuild();
  }
  setOutputPad(padding) {
    this.outputPad = padding;
    this.rebuild();
  }
  rebuild() {
    this.clear();
    this.addChild(new Markdown(this.text, this.outputPad, 1, this.markdownTheme, {
      color: (content) => theme_exports.theme.fg("userMessageText", content),
      bgColor: (content) => theme_exports.theme.bg("userMessageBg", content)
    }, {
      preserveOrderedListMarkers: true,
      preserveBackslashEscapes: true,
      transform: createMarkdownTransform("user", false, this.markdownTransformers)
    }));
  }
  render(width) {
    const lines = super.render(width);
    if (lines.length === 0) {
      return lines;
    }
    lines[0] = OSC133_ZONE_START2 + lines[0];
    lines[lines.length - 1] = OSC133_ZONE_END2 + OSC133_ZONE_FINAL2 + lines[lines.length - 1];
    return lines;
  }
};

// target/vendor-pi/dist/modes/interactive/components/user-message-selector.js
var UserMessageList = class {
  messages = [];
  selectedIndex = 0;
  onSelect;
  onCancel;
  maxVisible = 10;
  // Max messages visible
  constructor(messages, initialSelectedId) {
    this.messages = messages;
    const initialIndex = initialSelectedId ? messages.findIndex((message) => message.id === initialSelectedId) : -1;
    this.selectedIndex = initialIndex >= 0 ? initialIndex : Math.max(0, messages.length - 1);
  }
  invalidate() {
  }
  render(width) {
    const lines = [];
    if (this.messages.length === 0) {
      lines.push(theme_exports.theme.fg("muted", "  No user messages found"));
      return lines;
    }
    const startIndex = Math.max(0, Math.min(this.selectedIndex - Math.floor(this.maxVisible / 2), this.messages.length - this.maxVisible));
    const endIndex = Math.min(startIndex + this.maxVisible, this.messages.length);
    for (let i = startIndex; i < endIndex; i++) {
      const message = this.messages[i];
      const isSelected = i === this.selectedIndex;
      const normalizedMessage = message.text.replace(/\n/g, " ").trim();
      const cursor = isSelected ? theme_exports.theme.fg("accent", "\u203A ") : "  ";
      const maxMsgWidth = width - 2;
      const truncatedMsg = truncateToWidth(normalizedMessage, maxMsgWidth);
      const messageLine = cursor + (isSelected ? theme_exports.theme.bold(truncatedMsg) : truncatedMsg);
      lines.push(messageLine);
      const position = i + 1;
      const metadata = `  Message ${position} of ${this.messages.length}`;
      const metadataLine = theme_exports.theme.fg("muted", metadata);
      lines.push(metadataLine);
      lines.push("");
    }
    if (startIndex > 0 || endIndex < this.messages.length) {
      const scrollInfo = theme_exports.theme.fg("muted", `  (${this.selectedIndex + 1}/${this.messages.length})`);
      lines.push(scrollInfo);
    }
    return lines;
  }
  handleInput(keyData) {
    const kb = getKeybindings();
    if (kb.matches(keyData, "tui.select.up")) {
      this.selectedIndex = this.selectedIndex === 0 ? this.messages.length - 1 : this.selectedIndex - 1;
    } else if (kb.matches(keyData, "tui.select.down")) {
      this.selectedIndex = this.selectedIndex === this.messages.length - 1 ? 0 : this.selectedIndex + 1;
    } else if (kb.matches(keyData, "tui.select.confirm")) {
      const selected = this.messages[this.selectedIndex];
      if (selected && this.onSelect) {
        this.onSelect(selected.id);
      }
    } else if (kb.matches(keyData, "tui.select.cancel")) {
      if (this.onCancel) {
        this.onCancel();
      }
    }
  }
};
var UserMessageSelectorComponent = class extends Container {
  messageList;
  constructor(messages, onSelect, onCancel, initialSelectedId) {
    super();
    this.addChild(new Spacer(1));
    this.addChild(new Text(theme_exports.theme.bold("Fork from Message"), 1, 0));
    this.addChild(new Text(theme_exports.theme.fg("muted", "Select a user message to copy the active path up to that point into a new session"), 1, 0));
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder());
    this.addChild(new Spacer(1));
    this.messageList = new UserMessageList(messages, initialSelectedId);
    this.messageList.onSelect = onSelect;
    this.messageList.onCancel = onCancel;
    this.addChild(this.messageList);
    this.addChild(new Spacer(1));
    this.addChild(new DynamicBorder());
    if (messages.length === 0) {
      setTimeout(() => onCancel(), 100);
    }
  }
  getMessageList() {
    return this.messageList;
  }
};
export {
  ArminComponent,
  AssistantMessageComponent,
  BashExecutionComponent,
  BranchSummaryMessageComponent,
  CompactionSummaryMessageComponent,
  CustomMessageComponent,
  DEFAULT_MAX_BYTES,
  DEFAULT_MAX_LINES,
  ExtensionEditorComponent,
  ExtensionInputComponent,
  ExtensionSelectorComponent,
  FooterComponent,
  LoginDialogComponent,
  ModelSelectorComponent,
  OAuthSelectorComponent,
  ProjectTrustStore,
  SessionManager,
  SessionSelectorComponent,
  SettingsManager,
  SettingsSelectorComponent,
  ShowImagesSelectorComponent,
  SkillInvocationMessageComponent,
  ThemeSelectorComponent,
  ThinkingSelectorComponent,
  ToolExecutionComponent,
  TreeSelectorComponent,
  UserMessageComponent,
  UserMessageSelectorComponent,
  allToolNames,
  buildContextEntries,
  buildSessionContext,
  buildSessionProjection,
  calculateContextTokens,
  collectEntriesForBranchSummary,
  compact,
  createAllToolDefinitions,
  createAllTools,
  createBashTool,
  createBashToolDefinition,
  createCodingToolDefinitions,
  createCodingTools,
  createEditTool,
  createEditToolDefinition,
  createFindTool,
  createFindToolDefinition,
  createGrepTool,
  createGrepToolDefinition,
  createLocalBashOperations,
  createLocalPowerShellOperations,
  createLsTool,
  createLsToolDefinition,
  createPowerShellTool,
  createPowerShellToolDefinition,
  createReadOnlyToolDefinitions,
  createReadOnlyTools,
  createReadTool,
  createReadToolDefinition,
  createTool,
  createToolDefinition,
  createToolSearchExtension,
  createWriteTool,
  createWriteToolDefinition,
  detectSupportedImageMimeTypeFromFile,
  estimateTokens,
  findCutPoint,
  findTurnStartIndex,
  formatSize,
  formatSkillsForPrompt,
  generateBranchSummary,
  generateDiffString,
  generateSummary,
  generateSummaryWithUsage,
  generateUnifiedPatch,
  getLastAssistantUsage,
  getPowerShellConfig,
  hasTrustRequiringProjectResources,
  loadSkills,
  loadSkillsFromDir,
  migrateSessionEntries,
  parseArgs,
  parseSessionEntries,
  prepareBranchEntries,
  readStoredCredential,
  renderDiff,
  resolveCliModel,
  resolveModelScopeWithDiagnostics,
  sessionEntryToContextMessages,
  shouldCompact,
  truncateHead,
  truncateLine,
  truncateTail,
  withFileMutationQueue,
  wrapRegisteredTool,
  wrapRegisteredTools
};
