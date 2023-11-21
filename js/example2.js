import "java.nio.file.Files";
import "java.nio.file.Path";
import "java.nio.file.Paths";
import "java.util.Arrays";
import "com.google.gson.JsonObject";
import "java.util.Collections";
import "java.util.List";
import "java.util.stream.Collectors";
import "br.com.sankhya.ws.ServiceContext";
import JString from "java.lang.String";

var values = new JsonObject();
var res = new JsonObject();

var path = java.lang.System.getProperty("org.jboss.boot.log.file");

var filePath = Paths.get(path);
values.addProperty("filePath", filePath.toString());
var bytes = Files.readAllBytes(filePath);

var lines = new JString(bytes).split(System.lineSeparator());

list = Arrays.asList(lines);
Collections.reverse(list);
list = list.stream().limit(1000).collect(Collectors.toList());

var content = JString.join(System.lineSeparator(), list);
values.addProperty("content", content);
res.addProperty("log", content);

ServiceContext.getCurrent().setJsonResponse(res);
mensagem = content;
