#include "audit.hpp"
#include <algorithm>
#include <stdexcept>
namespace { std::string esc(std::string s){std::string o;for(char c:s){if(c=='\\'||c=='\"')o+='\\';o+=c;}return o;} class R:public Renderer{std::string f;std::vector<std::string>s;public:R(std::string x,std::vector<std::string> y):f(std::move(x)),s(std::move(y)){}std::string render(const Record&r)const override{std::string o=f=="json"?"{":"";for(size_t i=0;i<r.fields.size();++i){auto&[k,v]=r.fields[i];auto x=std::find(s.begin(),s.end(),k)!=s.end()?"***":v;if(f=="json"){if(i)o+=',';o+='\"'+esc(k)+"\":\""+esc(x)+"\"";}else{if(i)o+=' ';o+=k+'='+x;}}return f=="json"?o+"}":o;}};}
std::unique_ptr<Renderer> make_renderer(std::string_view f,std::vector<std::string>s){if(f!="text"&&f!="json")throw std::invalid_argument("format");return std::make_unique<R>(std::string(f),std::move(s));}
