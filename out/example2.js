const Files = Package.java.nio.file.Files;
const Path = Package.java.nio.file.Path;
const Paths = Package.java.nio.file.Paths;
const Arrays = Package.java.util.Arrays;
const JsonObject = Package.com.google.gson.JsonObject;
const Collections = Package.java.util.Collections;
const List = Package.java.util.List;
const Collectors = Package.java.util.stream.Collectors;
const ServiceContext = Package.br.com.sankhya.ws.ServiceContext;
const String = Package.java.lang.String
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
