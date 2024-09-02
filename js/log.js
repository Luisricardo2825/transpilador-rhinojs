import "br.com.sankhya.ws.ServiceContext";
import "com.google.gson.JsonObject";
import JString from "java.lang.String";
import "java.nio.file.Files";
import "java.nio.file.Path";
import "java.nio.file.Paths";
import "java.util.Arrays";
import "java.util.Collections";
import "java.util.List";
import "java.util.stream.Collectors";

let values = new JsonObject();
let res = new JsonObject();

let path = java.lang.System.getProperty("org.jboss.boot.log.file");

let filePath = Paths.get(path);
values.addProperty("filePath", filePath.toString());
let bytes = Files.readAllBytes(filePath);

let lines = new JString(bytes).split(System.lineSeparator());

list = Arrays.asList(lines);
Collections.reverse(list);
list = list.stream().limit(1000).collect(Collectors.toList());

let content = JString.join(System.lineSeparator(), list);
values.addProperty("content", content);
res.addProperty("log", content);

ServiceContext.getCurrent().setJsonResponse(res);
mensagem = content;
